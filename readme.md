# the spmm pim simulator
[![Build Status](https://app.travis-ci.com/shenjiangqiu/spmm_pim.svg?token=ZSTskApXkiDQtwDrTWeW&branch=master)](https://app.travis-ci.com/shenjiangqiu/spmm_pim)
[![codecov](https://codecov.io/gh/shenjiangqiu/spmm_pim/branch/master/graph/badge.svg?token=7qAtKQKmtx)](https://codecov.io/gh/shenjiangqiu/spmm_pim)
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
