// A dependency graph that contains any wasm must all be imported
// asynchronously. This `bootstrap.js` file does the single async import, so
// that no one else needs to worry about it again.
import("./index.js")
  .catch(e => console.error("Error importing `index.js`:", e)).then((index)=>{
     console.log("index.js loaded");
     document.getElementById("run").onclick =function(){
        this.innerHTML = "Running...";
         let result_list = index.run().then((x)=>{
            document.getElementById("result").innerHTML = x;

            this.innerHTML="click to run";

         });

     }
  });
