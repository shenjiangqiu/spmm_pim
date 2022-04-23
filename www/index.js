import * as wasm from "spmm_pim";

export async function run() {
    let para = document.getElementById("input").value;
    let result_list = [];
    if (para != "" && para != null) {
        console.log("clickbt:", para);
        await wasm.run1(para).then((x) => {
            console.log("wasm.run() returned:", x);
            result_list.push(x);
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
                result_list.push(x);
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
    return result_list;

}



