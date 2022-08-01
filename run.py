from multiprocessing import Pool
import subprocess
import workloads_defs




for inter in [1, 2,4,8,16]:
    for store in [32,64,128,256,512]:
        for graph in workloads:
            cmd = f"echo {graph};./gcn_sim -input=131072 -output=4194304 -edge=2097152 -hash-table-size={buffer_size} -agg=16777216 \
                -aggCores=512 -systolic-rows=32 -systolic-cols=128 -graph-name={graph} -dram-name={mem} \
                -model=gsc -ignore-neighbor=0 -ignore-self=0  \
                    -enable-feature-sparsity=0  -mem-sim={mem_sim} -dram-freq=0.5 -enable-dense-window -enable-fast-sched \
                        -short-large-divider={divider} -short-queue-size=100000 -large-queue-size=100000 -enable-ideal-selection  >{graph}.{buffer_size}.{divider}.sched.out 2>&1"
            print(cmd)
            cmds.append(cmd)


def run_task(command):
    subprocess.run(command, shell=True)


with Pool(int(10)) as p:
    p.map(run_task, cmds)