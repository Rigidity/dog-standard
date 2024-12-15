use chia::{
    clvm_traits::{FromClvm, ToClvm},
    protocol::{Bytes, Bytes32, Coin, CoinSpend},
};
use chia_wallet_sdk::{run_puzzle, Condition};
use clvmr::Allocator;

pub fn parse_memos(coin_spend: CoinSpend, coin_id: Bytes32) -> anyhow::Result<Option<Vec<Bytes>>> {
    let mut allocator = Allocator::new();
    let puzzle = coin_spend.puzzle_reveal.to_clvm(&mut allocator)?;
    let solution = coin_spend.solution.to_clvm(&mut allocator)?;
    let output = run_puzzle(&mut allocator, puzzle, solution)?;
    let conditions = Vec::<Condition>::from_clvm(&allocator, output)?;
    let create_coin = conditions
        .into_iter()
        .flat_map(Condition::into_create_coin)
        .find(|create_coin| {
            coin_id
                == Coin::new(
                    coin_spend.coin.coin_id(),
                    create_coin.puzzle_hash,
                    create_coin.amount,
                )
                .coin_id()
        });
    Ok(create_coin.map(|create_coin| create_coin.memos))
}
