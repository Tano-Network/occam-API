use sp1_build::build_program_with_args;

fn main() {
    build_program_with_args("../doge_tx", Default::default());
    build_program_with_args("../Xrp_tx", Default::default());
    build_program_with_args("../Xrp_balance", Default::default());
    build_program_with_args("../Cardano_tx", Default::default());
    build_program_with_args("../LiteCoin_tx", Default::default());
    build_program_with_args("../BitcoinCash_tx", Default::default());

    
    

}

