use chia::{
    clvm_traits::FromClvm,
    clvm_utils::{CurriedProgram, ToTreeHash, TreeHash},
    protocol::Bytes32,
};
use chia_wallet_sdk::{DriverError, Layer, Puzzle, SpendContext};
use clvmr::{Allocator, NodePtr};

use crate::puzzle::{DogArgs, DogSolution, DOG_PUZZLE, DOG_PUZZLE_HASH};

/// The DOG [`Layer`] enforces restrictions on the supply of a token.
/// Specifically, unless the TAIL program is run, the supply cannot change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DogLayer<I> {
    /// The amount of the DOG token.
    pub amount: u128,
    /// The asset id of the DOG token. This is the tree hash of the TAIL program.
    pub asset_id: Bytes32,
    /// The inner puzzle layer, commonly used for determining ownership.
    pub inner_puzzle: I,
}

impl<I> DogLayer<I> {
    pub fn new(amount: u128, asset_id: Bytes32, inner_puzzle: I) -> Self {
        Self {
            amount,
            asset_id,
            inner_puzzle,
        }
    }
}

impl<I> Layer for DogLayer<I>
where
    I: Layer,
{
    type Solution = DogSolution<I::Solution>;

    fn parse_puzzle(allocator: &Allocator, puzzle: Puzzle) -> Result<Option<Self>, DriverError> {
        let Some(puzzle) = puzzle.as_curried() else {
            return Ok(None);
        };

        if puzzle.mod_hash != DOG_PUZZLE_HASH {
            return Ok(None);
        }

        let args = DogArgs::<NodePtr>::from_clvm(allocator, puzzle.args)?;

        if args.mod_hash != DOG_PUZZLE_HASH.into() {
            return Err(DriverError::InvalidModHash);
        }

        let Some(inner_puzzle) =
            I::parse_puzzle(allocator, Puzzle::parse(allocator, args.inner_puzzle))?
        else {
            return Ok(None);
        };

        Ok(Some(Self {
            amount: args.amount,
            asset_id: args.asset_id,
            inner_puzzle,
        }))
    }

    fn parse_solution(
        allocator: &Allocator,
        solution: NodePtr,
    ) -> Result<Self::Solution, DriverError> {
        let solution = DogSolution::<NodePtr>::from_clvm(allocator, solution)?;
        let inner_solution = I::parse_solution(allocator, solution.inner_puzzle_solution)?;
        Ok(DogSolution {
            inner_puzzle_solution: inner_solution,
            lineage_proof: solution.lineage_proof,
            prev_coin_id: solution.prev_coin_id,
            this_coin_info: solution.this_coin_info,
            next_coin_proof: solution.next_coin_proof,
            prev_subtotal: solution.prev_subtotal,
            extra_delta: solution.extra_delta,
        })
    }

    fn construct_puzzle(&self, ctx: &mut SpendContext) -> Result<NodePtr, DriverError> {
        let ptr = ctx.puzzle(DOG_PUZZLE_HASH, &DOG_PUZZLE)?;
        let inner_puzzle = self.inner_puzzle.construct_puzzle(ctx)?;
        ctx.alloc(&CurriedProgram {
            program: ptr,
            args: DogArgs::new(self.amount, self.asset_id, inner_puzzle),
        })
    }

    fn construct_solution(
        &self,
        ctx: &mut SpendContext,
        solution: Self::Solution,
    ) -> Result<NodePtr, DriverError> {
        let inner_solution = self
            .inner_puzzle
            .construct_solution(ctx, solution.inner_puzzle_solution)?;
        ctx.alloc(&DogSolution {
            inner_puzzle_solution: inner_solution,
            lineage_proof: solution.lineage_proof,
            prev_coin_id: solution.prev_coin_id,
            this_coin_info: solution.this_coin_info,
            next_coin_proof: solution.next_coin_proof,
            prev_subtotal: solution.prev_subtotal,
            extra_delta: solution.extra_delta,
        })
    }
}

impl<I> ToTreeHash for DogLayer<I>
where
    I: ToTreeHash,
{
    fn tree_hash(&self) -> TreeHash {
        let inner_puzzle_hash = self.inner_puzzle.tree_hash();
        DogArgs::curry_tree_hash(self.amount, self.asset_id, inner_puzzle_hash)
    }
}

#[cfg(test)]
mod tests {
    use chia::protocol::Coin;
    use chia::puzzles::CoinProof;

    use super::*;

    #[test]
    fn test_cat_layer() -> anyhow::Result<()> {
        let mut ctx = SpendContext::new();
        let asset_id = Bytes32::new([1; 32]);

        let layer = DogLayer::new(0, asset_id, "Hello, world!".to_string());

        let ptr = layer.construct_puzzle(&mut ctx)?;
        let puzzle = Puzzle::parse(&ctx.allocator, ptr);
        let roundtrip =
            DogLayer::<String>::parse_puzzle(&ctx.allocator, puzzle)?.expect("invalid DOG layer");

        assert_eq!(roundtrip.asset_id, layer.asset_id);
        assert_eq!(roundtrip.inner_puzzle, layer.inner_puzzle);

        let expected = DogArgs::curry_tree_hash(0, asset_id, layer.inner_puzzle.tree_hash());
        assert_eq!(hex::encode(ctx.tree_hash(ptr)), hex::encode(expected));

        Ok(())
    }

    #[test]
    fn test_cat_solution() -> anyhow::Result<()> {
        let mut ctx = SpendContext::new();

        let layer = DogLayer::new(0, Bytes32::default(), NodePtr::NIL);

        let solution = DogSolution {
            inner_puzzle_solution: NodePtr::NIL,
            lineage_proof: None,
            prev_coin_id: Bytes32::default(),
            this_coin_info: Coin::new(Bytes32::default(), Bytes32::default(), 42),
            next_coin_proof: CoinProof {
                parent_coin_info: Bytes32::default(),
                inner_puzzle_hash: Bytes32::default(),
                amount: 34,
            },
            prev_subtotal: 0,
            extra_delta: 0,
        };
        let expected_ptr = ctx.alloc(&solution)?;
        let expected_hash = ctx.tree_hash(expected_ptr);

        let actual_ptr = layer.construct_solution(&mut ctx, solution)?;
        let actual_hash = ctx.tree_hash(actual_ptr);

        assert_eq!(hex::encode(actual_hash), hex::encode(expected_hash));

        let roundtrip = DogLayer::<NodePtr>::parse_solution(&ctx.allocator, actual_ptr)?;
        assert_eq!(roundtrip, solution);

        Ok(())
    }
}
