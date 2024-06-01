import { invoke } from "@tauri-apps/api/tauri";
import { RenderNodeProps } from "./render_node";
import { useState } from "react";
import RenderNode from "./render_node";

// how do I extract nodes from this window?
export default function NodeWindow() {
  const [nodes, setNodes] = useState(fetchNodes);

  function fetchNodes() {
    const initialNodes: RenderNodeProps[] = [];
    listNodes();
    return initialNodes;
  }

  function listNodes() {
    invoke("list_node").then((ctx: any) => setNodes(JSON.parse(ctx + "")));
  }

  function showDialog() {
    let dialog = document.getElementById("create_node");
    dialog?.showModal();
  }

  function closeDialog() {
    let dialog = document.getElementById("create_node");
    dialog?.close(); // how can I make this implicitly as dialog?
  }

  function handleSubmitNodeForm(e: any) {
    e.preventDefault();
    let data = {
      name: e.target.name.value,
      // TODO: remove magic hardcoded port value
      host: e.target.ip.value + ":15000",
    };
    invoke("create_node", data).then(listNodes);
    closeDialog();
  }

  function nodeWindow() {
    return (
      <div>
        {/* Show the activity of the computer progress */}
        <h2>Computer Nodes</h2>
        <button onClick={showDialog}>Connect</button>
        <div className="group" id="RenderNodes">
          {nodes.map(
            (node: RenderNodeProps, index: Number) => (
              (node.onDataChanged = listNodes), RenderNode(node)
            ),
          )}
        </div>
      </div>
    );
  }

  function nodeCreationDialog() {
    return (
      <dialog id="create_node">
        <form method="dialog" onSubmit={handleSubmitNodeForm}>
          <h1>Dialog</h1>
          {/* <label>Computer Name:</label>
          <input type="text" placeholder="Name" id="name" name="name" />
          <label>Internet Protocol Address</label>
          <input type="text" placeholder="IP Address" id="ip" name="ip" /> */}

          <menu>
            <button type="button" value="cancel" onClick={closeDialog}>
              Cancel
            </button>
            <button type="submit">Ok</button>
          </menu>
        </form>
      </dialog>
    );
  }

  return (
    <div>
      {/* I'm concern about dialog window size dimension */}
      {nodeWindow()}
      {nodeCreationDialog()}
    </div>
  );
}
