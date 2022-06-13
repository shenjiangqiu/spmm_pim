# the spmm pim simulator
[![github actions](https://img.shields.io/github/workflow/status/shenjiangqiu/spmm_pim/Rust?label=github-action)](https://github.com/shenjiangqiu/spmm_pim/actions/workflows/rust.yml)
[![Travis (.com)](https://img.shields.io/travis/com/shenjiangqiu/spmm_pim?label=travis-ci)](https://app.travis-ci.com/shenjiangqiu/spmm_pim)
[![codecov](https://img.shields.io/codecov/c/github/shenjiangqiu/spmm_pim)](https://codecov.io/gh/shenjiangqiu/spmm_pim)
## the configures for memory and graph are in configs file

## here are some important modules

 - bsr.rs: the core lib for bsr
 - result.rs: record the simulation statistics
 - settings.rs: read the configuration file for simulation 
 - run.rs, utils.rs,args.rs: utilitys to support in main.rs
 - lib.rs, the root library file 
 - main.rs, a cmd interface for SPMM

## the simulator is also availiable to be run in browser(thanks to the simple support for rust to compile the program to different target(like x64,arm and wasm))
### to run the simulator in browser: go to https://research.thesjq.com/spmm_pim/
