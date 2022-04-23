import * as wasm from "spmm_pim";

export async function run() {
    let para = document.getElementById("input").value;
    let result_list = [];
    let ok_list = [];
    let err_list = [];
    if (para != "" && para != null) {
        console.log("clickbt:", para);
        await wasm.run1(para).then((x) => {
            console.log("wasm.run() returned:", x);
            let js_x = JSON.parse(x);
            console.log("js_x:", js_x);
            result_list=result_list.concat(js_x.results);
            ok_list=ok_list.concat(js_x.ok_list);
            err_list=err_list.concat(js_x.err_list);
            // document.getElementById("result").innerHTML = x;
        }).catch((e) => {
            console.error("Error running `index.js`:", e);
            return;
        });
    } else {
        // find element by id
        let all_links = document.getElementById("file_list").children;
        for (let index = 0; index < all_links.length; index++) {
            const link = all_links[index];

            console.log("link:", link);
            console.log("link.innerHTML:", link.innerHTML);
            console.log("link.id:", link.id);
            await wasm.run1(link.innerHTML).then((x) => {
                link.className = "list-link-success";
                console.log("wasm.run() returned:", x);
                let js_x = JSON.parse(x);
                console.log("js_x:", js_x);

                result_list=result_list.concat(js_x.results);
                ok_list=ok_list.concat(js_x.ok_list);
                err_list=err_list.concat(js_x.err_list);
                // document.getElementById("result").innerHTML = x;
            }
            ).catch((e) => {
                link.className = "list-link-fail";
                console.error("Error running `index.js`:", e);
                return;
            }
            );

        }

    }
    console.log("result_list:", result_list);
    console.log("ok_list:", ok_list);
    console.log("err_list:", err_list);
    return {result_list:result_list,ok_list:ok_list,err_list:err_list};

}



