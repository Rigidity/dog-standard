use chia::{
    bls::PublicKey,
    clvm_traits::{clvm_quote, FromClvm},
    clvm_utils::CurriedProgram,
    protocol::{Bytes32, Coin},
    puzzles::{
        cat::{EverythingWithSignatureTailArgs, GenesisByCoinIdTailArgs},
        CoinProof, LineageProof,
    },
};
use chia_wallet_sdk::{
    run_puzzle, Condition, Conditions, CreateCoin, DriverError, Layer, Puzzle, Spend, SpendContext,
};
use clvmr::{Allocator, NodePtr};
use dog_layer::DogLayer;
use puzzle::{DogArgs, DogSolution};

mod dog_layer;
mod puzzle;

#[derive(Debug, Clone, Copy)]
pub struct DogSpend {
    pub dog: Dog,
    pub inner_spend: Spend,
    pub extra_delta: i64,
}

impl DogSpend {
    pub fn new(dog: Dog, inner_spend: Spend) -> Self {
        Self {
            dog,
            inner_spend,
            extra_delta: 0,
        }
    }

    pub fn with_extra_delta(dog: Dog, inner_spend: Spend, extra_delta: i64) -> Self {
        Self {
            dog,
            inner_spend,
            extra_delta,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SingleDogSpend {
    pub prev_coin_id: Bytes32,
    pub next_coin_proof: CoinProof,
    pub prev_subtotal: i64,
    pub extra_delta: i64,
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
            extra_delta: 0,
            inner_spend,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dog {
    pub coin: Coin,
    pub lineage_proof: Option<LineageProof>,
    pub amount: u128,
    pub asset_id: Bytes32,
    pub p2_puzzle_hash: Bytes32,
}

impl Dog {
    pub fn new(
        coin: Coin,
        lineage_proof: Option<LineageProof>,
        amount: u128,
        asset_id: Bytes32,
        p2_puzzle_hash: Bytes32,
    ) -> Self {
        Self {
            coin,
            lineage_proof,
            amount,
            asset_id,
            p2_puzzle_hash,
        }
    }

    pub fn single_issuance_eve(
        ctx: &mut SpendContext,
        parent_coin_id: Bytes32,
        amount: u128,
        extra_conditions: Conditions,
    ) -> Result<(Conditions, Dog), DriverError> {
        let ptr = ctx.genesis_by_coin_id_tail_puzzle()?;
        let tail = ctx.alloc(&CurriedProgram {
            program: ptr,
            args: GenesisByCoinIdTailArgs::new(parent_coin_id),
        })?;

        Self::create_and_spend_eve(
            ctx,
            parent_coin_id,
            ctx.tree_hash(tail).into(),
            amount,
            extra_conditions.run_cat_tail(tail, NodePtr::NIL),
        )
    }

    pub fn multi_issuance_eve(
        ctx: &mut SpendContext,
        parent_coin_id: Bytes32,
        public_key: PublicKey,
        amount: u128,
        extra_conditions: Conditions,
    ) -> Result<(Conditions, Dog), DriverError> {
        let ptr = ctx.everything_with_signature_tail_puzzle()?;
        let tail = ctx.alloc(&CurriedProgram {
            program: ptr,
            args: EverythingWithSignatureTailArgs::new(public_key),
        })?;

        Self::create_and_spend_eve(
            ctx,
            parent_coin_id,
            ctx.tree_hash(tail).into(),
            amount,
            extra_conditions.run_cat_tail(tail, NodePtr::NIL),
        )
    }

    /// Creates and spends an eve DOG with the provided conditions.
    /// To issue the DOG, you will need to reveal the TAIL puzzle and solution.
    /// This can be done with the [`RunCatTail`] condition.
    pub fn create_and_spend_eve(
        ctx: &mut SpendContext,
        parent_coin_id: Bytes32,
        asset_id: Bytes32,
        amount: u128,
        conditions: Conditions,
    ) -> Result<(Conditions, Dog), DriverError> {
        let inner_puzzle = ctx.alloc(&clvm_quote!(conditions))?;
        let eve_layer = DogLayer::new(amount, asset_id, inner_puzzle);
        let inner_puzzle_hash = ctx.tree_hash(inner_puzzle).into();
        let puzzle_ptr = eve_layer.construct_puzzle(ctx)?;
        let puzzle_hash = ctx.tree_hash(puzzle_ptr).into();

        let eve = Dog::new(
            Coin::new(parent_coin_id, puzzle_hash, 0),
            None,
            amount,
            asset_id,
            inner_puzzle_hash,
        );

        eve.spend(
            ctx,
            SingleDogSpend::eve(
                eve.coin,
                inner_puzzle_hash,
                Spend::new(inner_puzzle, NodePtr::NIL),
            ),
        )?;

        Ok((
            Conditions::new().create_coin(puzzle_hash, 0, Vec::new()),
            eve,
        ))
    }

    /// Creates coin spends for one or more DOGs in a ring.
    /// Without the ring announcements, DOG spends cannot share inputs and outputs.
    ///
    /// Each item is a DOG and the inner spend for that DOG.
    pub fn spend_all(ctx: &mut SpendContext, dog_spends: &[DogSpend]) -> Result<(), DriverError> {
        let len = dog_spends.len();

        let mut total_delta = 0;

        for (index, dog_spend) in dog_spends.iter().enumerate() {
            let DogSpend {
                dog,
                inner_spend,
                extra_delta,
            } = dog_spend;

            // Calculate the delta and add it to the subtotal.
            let output = ctx.run(inner_spend.puzzle, inner_spend.solution)?;
            let conditions: Vec<NodePtr> = ctx.extract(output)?;

            let create_coins = conditions
                .into_iter()
                .filter_map(|ptr| ctx.extract::<CreateCoin>(ptr).ok());

            let delta = create_coins.fold(
                i128::from(dog.coin.amount) - i128::from(*extra_delta),
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
                    prev_coin_id: prev.dog.coin.coin_id(),
                    next_coin_proof: CoinProof {
                        parent_coin_info: next.dog.coin.parent_coin_info,
                        inner_puzzle_hash: ctx.tree_hash(next.inner_spend.puzzle).into(),
                        amount: next.dog.coin.amount,
                    },
                    prev_subtotal: prev_subtotal.try_into()?,
                    extra_delta: *extra_delta,
                },
            )?;
        }

        Ok(())
    }

    /// Creates a coin spend for this DOG.
    pub fn spend(&self, ctx: &mut SpendContext, spend: SingleDogSpend) -> Result<(), DriverError> {
        let dog_layer = DogLayer::new(self.amount, self.asset_id, spend.inner_spend.puzzle);

        let puzzle = dog_layer.construct_puzzle(ctx)?;
        let solution = dog_layer.construct_solution(
            ctx,
            DogSolution {
                lineage_proof: self.lineage_proof,
                prev_coin_id: spend.prev_coin_id,
                this_coin_info: self.coin,
                next_coin_proof: spend.next_coin_proof,
                prev_subtotal: spend.prev_subtotal,
                extra_delta: spend.extra_delta,
                inner_puzzle_solution: spend.inner_spend.solution,
            },
        )?;

        ctx.spend(self.coin, Spend::new(puzzle, solution))
    }

    /// Returns the lineage proof that would be used by each child.
    pub fn child_lineage_proof(&self) -> LineageProof {
        LineageProof {
            parent_parent_coin_info: self.coin.parent_coin_info,
            parent_inner_puzzle_hash: self.p2_puzzle_hash,
            parent_amount: self.coin.amount,
        }
    }

    /// Creates a wrapped spendable DOG for a given output.
    #[must_use]
    pub fn wrapped_child(&self, p2_puzzle_hash: Bytes32, amount: u128) -> Self {
        let puzzle_hash =
            DogArgs::curry_tree_hash(self.amount, self.asset_id, p2_puzzle_hash.into());
        Self {
            coin: Coin::new(self.coin.coin_id(), puzzle_hash.into(), 0),
            lineage_proof: Some(self.child_lineage_proof()),
            amount,
            asset_id: self.asset_id,
            p2_puzzle_hash,
        }
    }
}

impl Dog {
    pub fn parse_children(
        allocator: &mut Allocator,
        parent_coin: Coin,
        parent_puzzle: Puzzle,
        parent_solution: NodePtr,
    ) -> Result<Option<Vec<Self>>, DriverError>
    where
        Self: Sized,
    {
        let Some(parent_layer) = DogLayer::<Puzzle>::parse_puzzle(allocator, parent_puzzle)? else {
            return Ok(None);
        };
        let parent_solution = DogLayer::<Puzzle>::parse_solution(allocator, parent_solution)?;

        let output = run_puzzle(
            allocator,
            parent_layer.inner_puzzle.ptr(),
            parent_solution.inner_puzzle_solution,
        )?;
        let conditions = Vec::<Condition>::from_clvm(allocator, output)?;

        let outputs = conditions
            .into_iter()
            .filter_map(Condition::into_create_coin)
            .map(|create_coin| {
                // Calculate what the wrapped puzzle hash would be for the created coin.
                // This is because we're running the inner layer.
                let wrapped_puzzle_hash = DogArgs::curry_tree_hash(
                    parent_layer.amount,
                    parent_layer.asset_id,
                    create_coin.puzzle_hash.into(),
                );

                Self {
                    coin: Coin::new(
                        parent_coin.coin_id(),
                        wrapped_puzzle_hash.into(),
                        create_coin.amount,
                    ),
                    lineage_proof: Some(LineageProof {
                        parent_parent_coin_info: parent_coin.parent_coin_info,
                        parent_inner_puzzle_hash: parent_layer
                            .inner_puzzle
                            .curried_puzzle_hash()
                            .into(),
                        parent_amount: parent_coin.amount,
                    }),
                    amount: create_coin.amount as u128,
                    asset_id: parent_layer.asset_id,
                    p2_puzzle_hash: create_coin.puzzle_hash,
                }
            })
            .collect();

        Ok(Some(outputs))
    }
}

#[cfg(test)]
mod tests {
    use chia::consensus::gen::validation_error::ErrorCode;
    use chia_wallet_sdk::{
        CreateCoin, Simulator, SimulatorError, SpendWithConditions, StandardLayer,
    };
    use rstest::rstest;

    use super::*;

    #[test]
    fn test_single_issuance_dog() -> anyhow::Result<()> {
        let mut sim = Simulator::new();
        let ctx = &mut SpendContext::new();
        let (sk, pk, puzzle_hash, coin) = sim.new_p2(1)?;
        let p2 = StandardLayer::new(pk);

        let (issue_dog, dog) = Dog::single_issuance_eve(
            ctx,
            coin.coin_id(),
            1,
            Conditions::new().create_coin(puzzle_hash, 1, vec![puzzle_hash.into()]),
        )?;
        p2.spend(ctx, coin, issue_dog)?;

        sim.spend_coins(ctx.take(), &[sk])?;

        let dog = dog.wrapped_child(puzzle_hash, 1);
        assert_eq!(dog.p2_puzzle_hash, puzzle_hash);
        assert_eq!(
            dog.asset_id,
            GenesisByCoinIdTailArgs::curry_tree_hash(coin.coin_id()).into()
        );
        assert!(sim.coin_state(dog.coin.coin_id()).is_some());

        Ok(())
    }

    #[test]
    fn test_multi_issuance_dog() -> anyhow::Result<()> {
        let mut sim = Simulator::new();
        let ctx = &mut SpendContext::new();
        let (sk, pk, puzzle_hash, coin) = sim.new_p2(1)?;
        let p2 = StandardLayer::new(pk);

        let (issue_dog, dog) = Dog::multi_issuance_eve(
            ctx,
            coin.coin_id(),
            pk,
            1,
            Conditions::new().create_coin(puzzle_hash, 1, vec![puzzle_hash.into()]),
        )?;
        p2.spend(ctx, coin, issue_dog)?;
        sim.spend_coins(ctx.take(), &[sk])?;

        let dog = dog.wrapped_child(puzzle_hash, 1);
        assert_eq!(dog.p2_puzzle_hash, puzzle_hash);
        assert_eq!(
            dog.asset_id,
            EverythingWithSignatureTailArgs::curry_tree_hash(pk).into()
        );
        assert!(sim.coin_state(dog.coin.coin_id()).is_some());

        Ok(())
    }

    #[test]
    fn test_missing_dog_issuance_output() -> anyhow::Result<()> {
        let mut sim = Simulator::new();
        let ctx = &mut SpendContext::new();
        let (sk, pk, _puzzle_hash, coin) = sim.new_p2(1)?;
        let p2 = StandardLayer::new(pk);

        let (issue_dog, _dog) =
            Dog::single_issuance_eve(ctx, coin.coin_id(), 1, Conditions::new())?;
        p2.spend(ctx, coin, issue_dog)?;

        assert!(matches!(
            sim.spend_coins(ctx.take(), &[sk]).unwrap_err(),
            SimulatorError::Validation(ErrorCode::AssertCoinAnnouncementFailed)
        ));

        Ok(())
    }

    #[test]
    fn test_exceeded_dog_issuance_output() -> anyhow::Result<()> {
        let mut sim = Simulator::new();
        let ctx = &mut SpendContext::new();
        let (sk, pk, puzzle_hash, coin) = sim.new_p2(2)?;
        let p2 = StandardLayer::new(pk);

        let (issue_dog, _dog) = Dog::single_issuance_eve(
            ctx,
            coin.coin_id(),
            1,
            Conditions::new().create_coin(puzzle_hash, 2, vec![puzzle_hash.into()]),
        )?;
        p2.spend(ctx, coin, issue_dog)?;

        assert!(matches!(
            sim.spend_coins(ctx.take(), &[sk]).unwrap_err(),
            SimulatorError::Validation(ErrorCode::AssertCoinAnnouncementFailed)
        ));

        Ok(())
    }

    #[rstest]
    #[case(1)]
    #[case(2)]
    #[case(3)]
    #[case(10)]
    fn test_dog_spends(#[case] coins: usize) -> anyhow::Result<()> {
        let mut sim = Simulator::new();
        let ctx = &mut SpendContext::new();

        // All of the amounts are different to prevent coin id collisions.
        let mut amounts = Vec::with_capacity(coins);

        for amount in 0..coins {
            amounts.push(amount as u64);
        }

        // Create the coin with the sum of all the amounts we need to issue.
        let sum = amounts.iter().sum::<u64>();
        let (sk, pk, puzzle_hash, coin) = sim.new_p2(sum)?;
        let p2 = StandardLayer::new(pk);

        // Issue the DOG coins with those amounts.
        let mut conditions = Conditions::new();

        for &amount in &amounts {
            conditions = conditions.create_coin(puzzle_hash, amount, vec![puzzle_hash.into()]);
        }

        let (issue_dog, dog) =
            Dog::single_issuance_eve(ctx, coin.coin_id(), sum as u128, conditions)?;
        p2.spend(ctx, coin, issue_dog)?;

        sim.spend_coins(ctx.take(), &[sk.clone()])?;

        let mut dogs: Vec<Dog> = amounts
            .into_iter()
            .map(|amount| dog.wrapped_child(puzzle_hash, amount as u128))
            .collect();

        // Spend the DOG coins a few times.
        for _ in 0..3 {
            let dog_spends: Vec<DogSpend> = dogs
                .iter()
                .map(|dog| {
                    Ok(DogSpend::new(
                        *dog,
                        p2.spend_with_conditions(
                            ctx,
                            Conditions::new().create_coin(
                                puzzle_hash,
                                dog.coin.amount,
                                vec![puzzle_hash.into()],
                            ),
                        )?,
                    ))
                })
                .collect::<anyhow::Result<_>>()?;

            Dog::spend_all(ctx, &dog_spends)?;
            sim.spend_coins(ctx.take(), &[sk.clone()])?;

            // Update the DOGs to the children.
            dogs = dogs
                .into_iter()
                .map(|dog| dog.wrapped_child(puzzle_hash, dog.coin.amount as u128))
                .collect();
        }

        Ok(())
    }

    #[test]
    fn test_different_dog_p2_puzzles() -> anyhow::Result<()> {
        let mut sim = Simulator::new();
        let ctx = &mut SpendContext::new();
        let (sk, pk, puzzle_hash, coin) = sim.new_p2(2)?;
        let p2 = StandardLayer::new(pk);

        // This will just return the solution verbatim.
        let custom_p2 = ctx.alloc(&1)?;
        let custom_p2_puzzle_hash = ctx.tree_hash(custom_p2).into();

        let (issue_dog, dog) = Dog::single_issuance_eve(
            ctx,
            coin.coin_id(),
            2,
            Conditions::new()
                .create_coin(puzzle_hash, 1, vec![puzzle_hash.into()])
                .create_coin(custom_p2_puzzle_hash, 1, vec![custom_p2_puzzle_hash.into()]),
        )?;
        p2.spend(ctx, coin, issue_dog)?;
        sim.spend_coins(ctx.take(), &[sk.clone()])?;

        let spends = [
            DogSpend::new(
                dog.wrapped_child(puzzle_hash, 1),
                p2.spend_with_conditions(
                    ctx,
                    Conditions::new().create_coin(puzzle_hash, 1, vec![puzzle_hash.into()]),
                )?,
            ),
            DogSpend::new(
                dog.wrapped_child(custom_p2_puzzle_hash, 1),
                Spend::new(
                    custom_p2,
                    ctx.alloc(&[CreateCoin::new(
                        custom_p2_puzzle_hash,
                        1,
                        vec![custom_p2_puzzle_hash.into()],
                    )])?,
                ),
            ),
        ];

        Dog::spend_all(ctx, &spends)?;
        sim.spend_coins(ctx.take(), &[sk])?;

        Ok(())
    }

    #[test]
    fn test_dog_melt() -> anyhow::Result<()> {
        let mut sim = Simulator::new();
        let ctx = &mut SpendContext::new();
        let (sk, pk, puzzle_hash, coin) = sim.new_p2(10000)?;
        let p2 = StandardLayer::new(pk);

        let conditions =
            Conditions::new().create_coin(puzzle_hash, 10000, vec![puzzle_hash.into()]);
        let (issue_dog, dog) = Dog::multi_issuance_eve(ctx, coin.coin_id(), pk, 10000, conditions)?;
        p2.spend(ctx, coin, issue_dog)?;

        let ptr = ctx.everything_with_signature_tail_puzzle()?;
        let tail = ctx.alloc(&CurriedProgram {
            program: ptr,
            args: EverythingWithSignatureTailArgs::new(pk),
        })?;

        let dog_spend = DogSpend::with_extra_delta(
            dog.wrapped_child(puzzle_hash, 10000),
            p2.spend_with_conditions(
                ctx,
                Conditions::new()
                    .create_coin(puzzle_hash, 7000, vec![puzzle_hash.into()])
                    .run_cat_tail(tail, NodePtr::NIL),
            )?,
            -3000,
        );

        Dog::spend_all(ctx, &[dog_spend])?;

        sim.spend_coins(ctx.take(), &[sk])?;

        Ok(())
    }
}
