import * as wasm from "spmm_pim";
import * as sjq from "./spmm"
export async function run() {
    console.log('run');
    sjq.default();
    let para = (document.getElementById("input") as HTMLInputElement).value;
    let result_list: any[] = [];
    let ok_list: string[] = [];
    let err_list: string[] = [];
    if (para != "" && para != null) {
        await wasm.run1(para).then((x) => {
            let js_x = JSON.parse(x);
            result_list = result_list.concat(js_x.results);
            ok_list = ok_list.concat(js_x.ok_list);
            err_list = err_list.concat(js_x.err_list);
            console.log("finished run single");
            // document.getElementById("result").innerHTML = x;
        }).catch((e) => {
            console.error("Error running `index.js`:", e);
            return;
        });
    } else {
        // find element by id
        let all_links = document.getElementById("file_list").children;
        for (let index = 0; index < all_links.length; index++) {
            console.log("running index: " + index);
            
            const link = all_links[index];


            await wasm.run1(link.innerHTML).then((x) => {
                link.className = "list-link-success";
                let js_x = JSON.parse(x);

                result_list = result_list.concat(js_x.results);
                ok_list = ok_list.concat(js_x.ok_list);
                err_list = err_list.concat(js_x.err_list);
                // document.getElementById("result").innerHTML = x;
            }
            ).catch((e) => {
                link.className = "list-link-fail";
                console.error("Error running `index.js`:", e);
                return;
            }
            );

        }
        console.log("finished run all");
        

    }

    return { result_list: result_list, ok_list: ok_list, err_list: err_list };

}



