import * as wasm from "spmm-pim";

export function clickbt() {
    let para=document.getElementById("input").value;
    console.log("clickbt:",para);
    wasm.run(para).then((x) => {
        console.log("wasm.run() returned:", x);
        document.getElementById("result").innerHTML = x;
    });
}



