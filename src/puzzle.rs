use chia::{
    clvm_traits::{self, FromClvm, ToClvm},
    clvm_utils::{CurriedProgram, ToTreeHash, TreeHash},
    protocol::Bytes32,
    puzzles::{CoinProof, LineageProof},
};
use hex_literal::hex;

pub const DOG_PUZZLE: [u8; 1022] = hex!("ff02ffff01ff04ffff04ff14ffff04ffff0114ffff04ff820bffffff04ffff02ff16ffff04ff02ffff04ff0bffff04ffff0bffff0101ff0b80ffff04ffff0bffff0101ff0580ffff04ffff0bffff0101ff6f80ffff04ffff0bffff0101ff4f80ff8080808080808080ff8080808080ffff04ffff04ff2cffff04ffff013fffff04ffff0eff10ffff0bff81bf8080ffff04ffff30ff8209ffffff02ff16ffff04ff02ffff04ff05ffff04ffff0bffff0101ff0580ffff04ffff0bffff0101ff0b80ffff04ff8215ffff80808080808080ff822dff80ff8080808080ffff04ffff04ff14ffff04ffff013fffff04ffff0eff10ffff0bff82017f8080ffff04ff8202ffff8080808080ffff02ff3effff04ff02ffff04ff05ffff04ff0bffff04ffff02ff17ff5f80ffff04ffff11ffff10ff820bffff82017f80ff81bf80ff80808080808080808080ffff04ffff01ffffff81d033ff43ff4202ffffff02ffff03ff05ffff01ff0bff81eaffff02ff2effff04ff02ffff04ff09ffff04ffff02ff12ffff04ff02ffff04ff0dff80808080ff808080808080ffff0181ca80ff0180ffffffa04bf5122f344554c53bde2ebb8cd2b7e3d1600ad631c385a5d7cce23c7785459aa09dcf97a184f32623d11a73124ceb99a5709b083721e878a16d78f596718ba7b2ffa102a12871fee210fb8619291eaea194581cbd2531e4b23759d225f6806923f63222a102a8d5dd63fba471ebcb1f3e8f7c1e1879b7152a6e7298a91ce119a63400ade7c5ff04ff18ffff04ffff02ff16ffff04ff02ffff04ff0bffff04ffff0bffff0101ff0b80ffff04ffff0bffff0101ff0580ffff04ffff0bffff0101ff5780ffff04ffff0bffff0101ff2780ff8080808080808080ffff04ff80ff77808080ffff0bff81aaffff02ff2effff04ff02ffff04ff05ffff04ffff02ff12ffff04ff02ffff04ff07ff80808080ff808080808080ffff0bff3cffff0bff3cff81caff0580ffff0bff3cff0bff818a8080ff02ffff03ff17ffff01ff02ffff03ffff09ff47ff1880ffff01ff04ffff02ff3affff04ff02ffff04ff05ffff04ff0bffff04ff67ff808080808080ffff02ff3effff04ff02ffff04ff05ffff04ff0bffff04ff37ffff04ffff11ff2fff82016780ff8080808080808080ffff01ff02ffff03ffff22ffff21ffff09ff47ff1480ffff09ff47ff2c8080ffff09ff81a7ffff013f80ffff09ffff0dff82016780ffff012180ffff09ffff0cff820167ff80ffff010180ff108080ffff01ff08ffff018378797a80ffff01ff04ff27ffff02ff3effff04ff02ffff04ff05ffff04ff0bffff04ff37ffff04ff2fff808080808080808080ff018080ff0180ffff01ff02ffff03ffff09ff2fff8080ff80ffff01ff08ffff01836162638080ff018080ff0180ff018080");
pub const DOG_PUZZLE_HASH: TreeHash = TreeHash::new(hex!(
    "cbdf86d15fada4046eebe75967646933389e1414955cf60fb436578cdc1145b2"
));

pub const DOG_LAUNCHER: [u8; 773] = hex!("ff02ffff01ff04ffff04ff14ffff04ffff0bff56ffff0bff12ffff0bff12ff66ff1780ffff0bff12ffff0bff76ffff0bff12ffff0bff12ff66ffff0bffff0101ff178080ffff0bff12ffff0bff76ffff0bff12ffff0bff12ff66ffff0bffff0101ff0b8080ffff0bff12ffff0bff76ffff0bff12ffff0bff12ff66ff5f80ffff0bff12ff66ff46808080ff46808080ff46808080ff46808080ffff04ffff10ff2fffff02ffff03ff81bfffff0182013fff8080ff018080ff80808080ffff04ffff04ff10ffff04ff8202ffff808080ffff04ffff04ff3cffff04ffff0114ffff04ffff10ff2fffff02ffff03ff81bfffff0182013fff8080ff018080ffff04ff8202ffff8080808080ffff04ffff02ffff03ffff22ffff09ff2fff8080ff81bf80ffff01ff04ff2cff8080ffff01ff04ff18ffff04ffff30ff82027fffff02ff2effff04ff02ffff04ff17ffff04ffff0bffff0101ff1780ffff04ffff0bffff0101ff0b80ffff04ff82057fff80808080808080ff820b7f80ff80808080ff0180ffff02ffff03ff81bfffff01ff02ff8202bfffff04ffff04ff05ffff04ff0bffff04ff17ffff04ff2fffff04ff5fffff04ff82013fffff04ff8202ffff8080808080808080ff8203bf8080ff8080ff018080808080ffff04ffff01ffffff4647ff33ff0142ffff02ff02ffff03ff05ffff01ff0bff76ffff02ff3effff04ff02ffff04ff09ffff04ffff02ff1affff04ff02ffff04ff0dff80808080ff808080808080ffff016680ff0180ffffffa04bf5122f344554c53bde2ebb8cd2b7e3d1600ad631c385a5d7cce23c7785459aa09dcf97a184f32623d11a73124ceb99a5709b083721e878a16d78f596718ba7b2ffa102a12871fee210fb8619291eaea194581cbd2531e4b23759d225f6806923f63222a102a8d5dd63fba471ebcb1f3e8f7c1e1879b7152a6e7298a91ce119a63400ade7c5ffff0bff56ffff02ff3effff04ff02ffff04ff05ffff04ffff02ff1affff04ff02ffff04ff07ff80808080ff808080808080ff0bff12ffff0bff12ff66ff0580ffff0bff12ff0bff468080ff018080");
pub const DOG_LAUNCHER_HASH: TreeHash = TreeHash::new(hex!(
    "cd4d3dd1d1b9a30ac5b1b443795c6070b5fc36b9599bf21a153995db7731d57b"
));

#[derive(Debug, Clone, Copy, PartialEq, Eq, ToClvm, FromClvm)]
#[clvm(curry)]
pub struct DogArgs<I> {
    pub mod_hash: Bytes32,
    pub launcher_self_hash: Bytes32,
    pub inner_puzzle: I,
}

impl<I> DogArgs<I> {
    pub fn new(asset_id: Bytes32, inner_puzzle: I) -> Self {
        Self {
            mod_hash: DOG_PUZZLE_HASH.into(),
            launcher_self_hash: DogLauncherSelfArgs::curry_tree_hash(asset_id).into(),
            inner_puzzle,
        }
    }
}

impl DogArgs<TreeHash> {
    pub fn curry_tree_hash(asset_id: Bytes32, inner_puzzle: TreeHash) -> TreeHash {
        CurriedProgram {
            program: DOG_PUZZLE_HASH,
            args: Self::new(asset_id, inner_puzzle),
        }
        .tree_hash()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ToClvm, FromClvm)]
#[clvm(list)]
pub struct LauncherProof {
    pub parent_inner_puzzle_hash: Bytes32,
    #[clvm(rest)]
    pub parent_amount: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ToClvm, FromClvm)]
#[clvm(solution)]
pub struct DogSolution<I> {
    pub launcher_proof: LauncherProof,
    pub inner_solution: I,
    pub next_coin_delta: i64,
    pub prev_coin_delta: i64,
    pub prev_coin_id: Bytes32,
    pub next_coin_proof: CoinProof,
    pub my_amount: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ToClvm, FromClvm)]
#[clvm(curry)]
pub struct DogLauncherSelfArgs {
    pub asset_id: Bytes32,
}

impl DogLauncherSelfArgs {
    pub fn new(asset_id: Bytes32) -> Self {
        Self { asset_id }
    }

    pub fn curry_tree_hash(asset_id: Bytes32) -> TreeHash {
        CurriedProgram {
            program: DOG_LAUNCHER_HASH,
            args: Self::new(asset_id),
        }
        .tree_hash()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ToClvm, FromClvm)]
#[clvm(curry)]
pub struct DogLauncherArgs {
    pub launcher_self_hash: Bytes32,
    pub dog_mod_hash: Bytes32,
    pub amount: u64,
    pub inner_puzzle_hash: Bytes32,
}

impl DogLauncherArgs {
    pub fn new_outer(asset_id: Bytes32, amount: u64, inner_puzzle_hash: Bytes32) -> Self {
        Self {
            launcher_self_hash: DogLauncherSelfArgs::curry_tree_hash(asset_id).into(),
            dog_mod_hash: DOG_PUZZLE_HASH.into(),
            amount,
            inner_puzzle_hash,
        }
    }
}

impl DogLauncherArgs {
    pub fn curry_tree_hash(asset_id: Bytes32, amount: u64, inner_puzzle_hash: Bytes32) -> TreeHash {
        CurriedProgram {
            program: Bytes32::from(DogLauncherSelfArgs::curry_tree_hash(asset_id)).tree_hash(),
            args: Self::new_outer(asset_id, amount, inner_puzzle_hash),
        }
        .tree_hash()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ToClvm, FromClvm)]
#[clvm(list)]
pub struct TailPack<P, S> {
    pub delta: i64,
    pub tail_reveal: P,
    #[clvm(rest)]
    pub tail_solution: S,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ToClvm, FromClvm)]
#[clvm(solution)]
pub struct DogLauncherSolution<P, S> {
    pub tail_pack: Option<TailPack<P, S>>,
    pub lineage_proof: Option<LineageProof>,
    pub my_id: Bytes32,
}
