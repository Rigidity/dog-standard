use chia::{
    clvm_traits::{self, FromClvm, ToClvm},
    clvm_utils::{CurriedProgram, ToTreeHash, TreeHash},
    protocol::{Bytes32, Coin},
    puzzles::{CoinProof, LineageProof},
};
use hex_literal::hex;

pub const DOG_PUZZLE: [u8; 1701] = hex!("ff02ffff01ff02ff5effff04ff02ffff04ffff04ff05ffff04ffff0bffff0101ff0580ffff04ff17ff80808080ffff04ffff02ff2fff5f80ffff04ff81bfffff04ffff02ff2effff04ff02ffff04ff2fff80808080ffff04ffff30ff8204ffff820affff8216ff80ffff04ff82017fffff04ff8202ffffff04ff8205ffffff04ff820bffffff04ff8217ffff80808080808080808080808080ffff04ffff01ffffffff3d46ff333cffff81cb02ffff02ffff03ff05ffff01ff0bff81f2ffff02ff26ffff04ff02ffff04ff09ffff04ffff02ff2cffff04ff02ffff04ff0dff80808080ff808080808080ffff0181d280ff0180ff02ffff03ff0bffff01ff02ffff03ffff09ffff02ff2effff04ff02ffff04ff13ff80808080ff820b9f80ffff01ff02ff36ffff04ff02ffff04ffff02ff13ffff04ff5fffff04ff17ffff04ff2fffff04ff81bfffff04ff82017fffff04ff1bff8080808080808080ffff04ff82017fff8080808080ffff01ff088080ff0180ffff01ff02ffff03ff17ffff01ff02ffff03ffff20ff81bf80ffff0182017fffff01ff088080ff0180ffff01ff088080ff018080ff0180ffffffff04ffff04ff05ff2780ffff04ffff10ff0bff5780ff778080ffffa04bf5122f344554c53bde2ebb8cd2b7e3d1600ad631c385a5d7cce23c7785459aa09dcf97a184f32623d11a73124ceb99a5709b083721e878a16d78f596718ba7b2ffa102a12871fee210fb8619291eaea194581cbd2531e4b23759d225f6806923f63222a102a8d5dd63fba471ebcb1f3e8f7c1e1879b7152a6e7298a91ce119a63400ade7c5ffff0bff81b2ffff02ff26ffff04ff02ffff04ff05ffff04ffff02ff2cffff04ff02ffff04ff07ff80808080ff808080808080ff02ffff03ff05ffff01ff02ffff03ffff09ffff02ffff03ffff09ff11ff2880ffff0159ff8080ff0180ffff01818f80ffff01ff02ff3affff04ff02ffff04ff0dffff04ff0bffff04ffff04ff81b9ff82017980ff808080808080ffff01ff02ff22ffff04ff02ffff04ffff02ffff03ffff09ff11ff2880ffff01ff04ff28ffff04ffff02ff2affff04ff02ffff04ff13ffff04ff2bffff04ffff0bffff0101ff5980ffff04ffff0bffff0101ff5b80ffff04ff29ff8080808080808080ffff04ff80ff79808080ffff01ff02ffff03ffff09ff11ff3880ffff01ff02ffff03ffff20ffff02ffff03ffff09ffff0121ffff0dff298080ffff01ff02ffff03ffff09ffff0cff29ff80ffff010180ff2480ffff01ff0101ff8080ff0180ff8080ff018080ffff0109ffff01ff088080ff0180ffff010980ff018080ff0180ffff04ffff02ffff03ffff09ff11ff2880ffff0159ff8080ff0180ffff04ffff02ff3affff04ff02ffff04ff0dffff04ff0bffff04ff17ff808080808080ff80808080808080ff0180ffff01ff04ff80ffff04ff80ff17808080ff0180ffffff0bff34ffff0bff34ff81d2ff0580ffff0bff34ff0bff81928080ff02ffff03ff05ffff01ff04ff09ffff02ff36ffff04ff02ffff04ff0dffff04ff0bff808080808080ffff010b80ff0180ffff02ffff03ffff07ff0580ffff01ff0bffff0102ffff02ff2effff04ff02ffff04ff09ff80808080ffff02ff2effff04ff02ffff04ff0dff8080808080ffff01ff0bffff0101ff058080ff0180ffff04ffff04ff30ffff04ff5fff808080ffff02ff7effff04ff02ffff04ffff04ffff04ff2fff0580ffff04ff5fff82017f8080ffff04ffff02ff3affff04ff02ffff04ff0bffff04ff05ffff01ff808080808080ffff04ff17ffff04ff81bfffff04ff82017fffff04ffff30ff8204ffffff02ff2affff04ff02ffff04ff09ffff04ff15ffff04ffff0bffff0101ff8216ff80ffff04ffff0bffff0101ff2d80ffff04ff820affff8080808080808080ff8080ffff04ff8205ffffff04ff820bffff808080808080808080808080ff02ff3cffff04ff02ffff04ff5fffff04ff3bffff04ffff02ffff03ff17ffff01ff09ff2dffff30ff27ffff02ff2affff04ff02ffff04ff29ffff04ff59ffff04ffff0bffff0101ff81b780ffff04ffff0bffff0101ff81b980ffff04ff57ff8080808080808080ff81b78080ff8080ff0180ffff04ff17ffff04ff05ffff04ff8202ffffff04ffff04ffff04ff38ffff04ffff0eff24ffff02ff2effff04ff02ffff04ffff04ff2fffff04ff82017fff808080ff8080808080ff808080ffff04ffff04ff20ffff04ffff0bff81bfff24ffff02ff2effff04ff02ffff04ffff04ff15ffff04ffff10ff82017fffff11ff8202dfff2b80ff8202ff80ff808080ff8080808080ff808080ff138080ff80808080808080808080ff018080");
pub const DOG_PUZZLE_HASH: TreeHash = TreeHash::new(hex!(
    "a7d7c35be0c9f6b6b743fc386bc635af8b42b18cc11e77359f94abf9ba1f92d0"
));

#[derive(Debug, Clone, Copy, PartialEq, Eq, ToClvm, FromClvm)]
#[clvm(curry)]
pub struct DogArgs<I> {
    pub mod_hash: Bytes32,
    pub amount: u128,
    pub asset_id: Bytes32,
    pub inner_puzzle: I,
}

impl<I> DogArgs<I> {
    pub fn new(amount: u128, asset_id: Bytes32, inner_puzzle: I) -> Self {
        Self {
            mod_hash: DOG_PUZZLE_HASH.into(),
            amount,
            asset_id,
            inner_puzzle,
        }
    }
}

impl DogArgs<TreeHash> {
    pub fn curry_tree_hash(amount: u128, asset_id: Bytes32, inner_puzzle: TreeHash) -> TreeHash {
        CurriedProgram {
            program: DOG_PUZZLE_HASH,
            args: DogArgs {
                mod_hash: DOG_PUZZLE_HASH.into(),
                amount,
                asset_id,
                inner_puzzle,
            },
        }
        .tree_hash()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ToClvm, FromClvm)]
#[clvm(solution)]
pub struct DogSolution<I> {
    pub inner_puzzle_solution: I,
    pub lineage_proof: Option<LineageProof>,
    pub prev_coin_id: Bytes32,
    pub this_coin_info: Coin,
    pub next_coin_proof: CoinProof,
    pub prev_subtotal: i64,
    pub extra_delta: i64,
}
