use chia::{
    clvm_utils::CurriedProgram,
    protocol::{Bytes32, Coin},
    puzzles::{CoinProof, LineageProof},
};
use chia_wallet_sdk::{CreateCoin, DriverError, Spend, SpendContext};
use clvmr::NodePtr;
use puzzle::{
    DogArgs, DogLauncherArgs, DogLauncherSelfArgs, DogLauncherSolution, DogSolution, LauncherProof,
    DOG_LAUNCHER, DOG_LAUNCHER_HASH, DOG_PUZZLE, DOG_PUZZLE_HASH,
};

mod puzzle;

#[derive(Debug, Clone, Copy)]
pub struct DogSpend {
    pub dog: Dog,
    pub inner_spend: Spend,
}

impl DogSpend {
    pub fn new(dog: Dog, inner_spend: Spend) -> Self {
        Self { dog, inner_spend }
    }

    pub fn with_extra_delta(dog: Dog, inner_spend: Spend) -> Self {
        Self { dog, inner_spend }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SingleDogSpend {
    pub prev_coin_id: Bytes32,
    pub next_coin_proof: CoinProof,
    pub prev_subtotal: i64,
    pub inner_spend: Spend,
}

impl SingleDogSpend {
    pub fn eve(coin: Coin, inner_puzzle_hash: Bytes32, inner_spend: Spend) -> Self {
        Self {
            prev_coin_id: coin.coin_id(),
            next_coin_proof: CoinProof {
                parent_coin_info: coin.parent_coin_info,
                inner_puzzle_hash,
                amount: coin.amount,
            },
            prev_subtotal: 0,
            inner_spend,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dog {
    pub launcher_coin: Coin,
    pub ephemeral_coin: Coin,
    pub lineage_proof: LineageProof,
    pub asset_id: Bytes32,
    pub p2_puzzle_hash: Bytes32,
}

impl Dog {
    pub fn spend_all(ctx: &mut SpendContext, dog_spends: &[DogSpend]) -> Result<(), DriverError> {
        let len = dog_spends.len();

        let mut total_delta = 0;

        for (index, dog_spend) in dog_spends.iter().enumerate() {
            let DogSpend { dog, inner_spend } = dog_spend;

            // Calculate the delta and add it to the subtotal.
            let output = ctx.run(inner_spend.puzzle, inner_spend.solution)?;
            let conditions: Vec<NodePtr> = ctx.extract(output)?;

            let create_coins = conditions
                .into_iter()
                .filter_map(|ptr| ctx.extract::<CreateCoin>(ptr).ok());

            let delta = create_coins.fold(
                i128::from(dog.ephemeral_coin.amount),
                |delta, create_coin| delta - i128::from(create_coin.amount),
            );

            let prev_subtotal = total_delta;
            total_delta += delta;

            // Find information of neighboring coins on the ring.
            let prev = &dog_spends[if index == 0 { len - 1 } else { index - 1 }];
            let next = &dog_spends[if index == len - 1 { 0 } else { index + 1 }];

            dog.spend(
                ctx,
                SingleDogSpend {
                    inner_spend: *inner_spend,
                    prev_coin_id: prev.dog.ephemeral_coin.coin_id(),
                    next_coin_proof: CoinProof {
                        parent_coin_info: next.dog.ephemeral_coin.parent_coin_info,
                        inner_puzzle_hash: ctx.tree_hash(next.inner_spend.puzzle).into(),
                        amount: next.dog.ephemeral_coin.amount,
                    },
                    prev_subtotal: prev_subtotal.try_into()?,
                },
            )?;
        }

        Ok(())
    }

    /// Creates a coin spend for this DOG.
    pub fn spend(&self, ctx: &mut SpendContext, spend: SingleDogSpend) -> Result<(), DriverError> {
        let launcher_mod = ctx.puzzle(DOG_LAUNCHER_HASH, &DOG_LAUNCHER)?;
        let dog_mod = ctx.puzzle(DOG_PUZZLE_HASH, &DOG_PUZZLE)?;

        let launcher_puzzle = ctx.alloc(&CurriedProgram {
            program: CurriedProgram {
                program: launcher_mod,
                args: DogLauncherSelfArgs::new(self.asset_id),
            },
            args: DogLauncherArgs::new_outer(
                self.asset_id,
                self.ephemeral_coin.amount,
                self.p2_puzzle_hash,
            ),
        })?;

        let launcher_solution = ctx.alloc(&DogLauncherSolution::<NodePtr, NodePtr> {
            tail_pack: None,
            lineage_proof: Some(self.lineage_proof),
            my_id: self.launcher_coin.coin_id(),
        })?;

        let puzzle = ctx.alloc(&CurriedProgram {
            program: dog_mod,
            args: DogArgs::new(self.asset_id, spend.inner_spend.puzzle),
        })?;

        let solution = ctx.alloc(&DogSolution {
            launcher_proof: LauncherProof {
                parent_inner_puzzle_hash: self.lineage_proof.parent_inner_puzzle_hash,
                parent_amount: self.lineage_proof.parent_amount,
            },
            inner_solution: spend.inner_spend.solution,
            next_coin_delta: 0,
            prev_coin_delta: 0,
            prev_coin_id: spend.prev_coin_id,
            next_coin_proof: spend.next_coin_proof,
            my_amount: self.ephemeral_coin.amount,
        })?;

        println!(
            "Launcher: {:?} {:?}",
            ctx.serialize(&launcher_puzzle)?,
            ctx.serialize(&launcher_solution)?
        );
        println!(
            "Ephemeral: {:?} {:?}",
            ctx.serialize(&puzzle)?,
            ctx.serialize(&solution)?
        );

        ctx.spend(
            self.launcher_coin,
            Spend::new(launcher_puzzle, launcher_solution),
        )?;
        ctx.spend(self.ephemeral_coin, Spend::new(puzzle, solution))?;

        Ok(())
    }

    /// Returns the lineage proof that would be used by each child.
    pub fn child_lineage_proof(&self) -> LineageProof {
        LineageProof {
            parent_parent_coin_info: self.ephemeral_coin.parent_coin_info,
            parent_inner_puzzle_hash: self.p2_puzzle_hash,
            parent_amount: self.ephemeral_coin.amount,
        }
    }

    /// Creates a wrapped spendable DOG for a given output.
    #[must_use]
    pub fn wrapped_child(&self, p2_puzzle_hash: Bytes32, amount: u64) -> Self {
        let launcher_puzzle_hash =
            DogLauncherArgs::curry_tree_hash(self.asset_id, amount, p2_puzzle_hash);
        let ephemeral_puzzle_hash = DogArgs::curry_tree_hash(self.asset_id, p2_puzzle_hash.into());

        let launcher_coin = Coin::new(
            self.ephemeral_coin.coin_id(),
            launcher_puzzle_hash.into(),
            0,
        );

        Self {
            launcher_coin,
            ephemeral_coin: Coin::new(
                launcher_coin.coin_id(),
                ephemeral_puzzle_hash.into(),
                amount,
            ),
            lineage_proof: self.child_lineage_proof(),
            asset_id: self.asset_id,
            p2_puzzle_hash,
        }
    }
}

#[cfg(test)]
mod tests {
    use chia::clvm_utils::ToTreeHash;
    use chia_wallet_sdk::{Conditions, Simulator, SpendWithConditions, StandardLayer};
    use puzzle::TailPack;

    use super::*;

    #[test]
    fn test_single_issuance_dog() -> anyhow::Result<()> {
        let mut sim = Simulator::new();
        let ctx = &mut SpendContext::new();
        let (sk, pk, puzzle_hash, coin) = sim.new_p2(1)?;
        let p2 = StandardLayer::new(pk);

        let tail = ();
        let asset_id = tail.tree_hash().into();

        let launcher_puzzle_hash =
            DogLauncherArgs::curry_tree_hash(asset_id, 0, puzzle_hash).into();

        p2.spend(
            ctx,
            coin,
            Conditions::new().create_coin(launcher_puzzle_hash, 0, Vec::new()),
        )?;
        let launcher_coin = Coin::new(coin.coin_id(), launcher_puzzle_hash, 0);
        let dog_puzzle_hash = DogArgs::curry_tree_hash(asset_id, puzzle_hash.into()).into();
        let ephemeral_coin = Coin::new(launcher_coin.coin_id(), dog_puzzle_hash, 1);

        sim.spend_coins(ctx.take(), &[sk.clone()])?;

        let launcher_mod = ctx.puzzle(DOG_LAUNCHER_HASH, &DOG_LAUNCHER)?;
        let dog_mod = ctx.puzzle(DOG_PUZZLE_HASH, &DOG_PUZZLE)?;

        let launcher_puzzle = ctx.alloc(&CurriedProgram {
            program: CurriedProgram {
                program: launcher_mod,
                args: DogLauncherSelfArgs::new(asset_id),
            },
            args: DogLauncherArgs::new_outer(asset_id, 0, puzzle_hash),
        })?;

        let launcher_solution = ctx.alloc(&DogLauncherSolution::<NodePtr, NodePtr> {
            tail_pack: Some(TailPack {
                delta: 1,
                tail_reveal: NodePtr::NIL,
                tail_solution: NodePtr::NIL,
            }),
            lineage_proof: None,
            my_id: launcher_coin.coin_id(),
        })?;

        let spend = p2.spend_with_conditions(
            ctx,
            Conditions::new().create_coin([0; 32].into(), 1, Vec::new()),
        )?;

        let puzzle = ctx.alloc(&CurriedProgram {
            program: dog_mod,
            args: DogArgs::new(asset_id, spend.puzzle),
        })?;

        let solution = ctx.alloc(&DogSolution {
            launcher_proof: LauncherProof {
                parent_inner_puzzle_hash: puzzle_hash,
                parent_amount: coin.amount,
            },
            inner_solution: spend.solution,
            next_coin_delta: 0,
            prev_coin_delta: 0,
            prev_coin_id: ephemeral_coin.coin_id(),
            next_coin_proof: CoinProof {
                parent_coin_info: ephemeral_coin.parent_coin_info,
                inner_puzzle_hash: puzzle_hash,
                amount: ephemeral_coin.amount,
            },
            my_amount: ephemeral_coin.amount,
        })?;

        println!(
            "Launcher: {:?} {:?}",
            ctx.serialize(&launcher_puzzle)?,
            ctx.serialize(&launcher_solution)?
        );
        println!("Launcher coin id: {:?}", launcher_coin.coin_id());
        // println!(
        //     "Ephemeral: {:?} {:?}",
        //     ctx.serialize(&puzzle)?,
        //     ctx.serialize(&solution)?
        // );

        ctx.spend(
            launcher_coin,
            Spend::new(launcher_puzzle, launcher_solution),
        )?;
        ctx.spend(ephemeral_coin, Spend::new(puzzle, solution))?;

        sim.spend_coins(ctx.take(), &[sk])?;

        Ok(())
    }
}
