use sp1_build::build_program_with_args;

fn main() {
    build_program_with_args("../program", Default::default());
    build_program_with_args("../Icr", Default::default());
    build_program_with_args("../Liquid", Default::default());
    build_program_with_args("../Real_time_ltv", Default::default());
    build_program_with_args("../doge_tx", Default::default());

    
    
    build_program_with_args("../BimaOccumPutCall", Default::default());

}
