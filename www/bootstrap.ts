// A dependency graph that contains any wasm must all be imported
// asynchronously. This `bootstrap.js` file does the single async import, so
// that no one else needs to worry about it again.
import("./index")
   .catch(e => console.error("Error importing `index.js`:", e)).then((index) => {
      console.log("index.js loaded");
      (document.getElementById("run")).onclick = function () {
         let el = this as HTMLButtonElement;
         el.innerHTML = "Running...";
         let indext = index as typeof import("./index");
         let result_list = indext.run().then((x) => {
            document.getElementById("result").innerHTML = JSON.stringify({ result_list: x.result_list });
            document.getElementById("ok_list").innerHTML = JSON.stringify({ ok_list: new Set(x.ok_list) });
            document.getElementById("err_list").innerHTML = JSON.stringify({ err_list: new Set(x.err_list) });

            el.innerHTML = "click to run";

         });

      }
   });
