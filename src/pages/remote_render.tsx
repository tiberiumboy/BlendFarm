import { invoke } from "@tauri-apps/api/tauri";
import { useState } from "react";
import RenderNode, { RenderNodeProps } from "../components/render_node";

export default function RemoteRender() {
  
  const [nodes, setNodes] = useState([]);

  function CreateNewJob(e:any) {
    e.preventDefault();
    invoke("create_job");
    return false;
  }

  function handleSubmit(e: any) {
    e.preventDefault();
    let data = {
      name: e.target.name.value,
      host: e.target.ip.value + ":15000"
    };
    invoke("create_node", data).then(listNode);
    closeCreateNew("create_node");
  }

  function listNode() {
    invoke("list_node").then((ctx) => {
      let data = JSON.parse(ctx + "");
      setNodes(data);
    });
  }

  function deleteNode(id: any) {
    invoke("delete_node", { id }).then(() => {
      let col = nodes.filter((node: any) => node.id != id);
      setNodes(col);
    });
  }

  function showCreateNew(id: string) {
    let dialog = document.getElementById(id);
    dialog.showModal();
  }

  function closeCreateNew(id: string) {
    let dialog = document.getElementById(id);
    dialog.close();
  }

  return (
    <div className="content">
      <h1>Remote Render</h1>

      {/* Show the activity of the computer progress */}
      <h2>Computer Nodes</h2>
      <button onClick={() => showCreateNew("create_node")}>Create New</button>
      <br></br>
      <h4>
        <div className="group" id="RenderNodes">
          {nodes.map((node: RenderNodeProps) => (
            // A bit far fetch here, but can we rename nodes? Or edit it?
            <RenderNode
              id={node.id}
              key={node.id}
              name={node.name}
              onDataChanged={listNode}
            />
          ))}
        </div>
      </h4>

      

      {/* Collection of active job list */}
      <h2>Job List</h2>

      <button onClick={CreateNewJob}>Create new Job</button>
      <p>
        This is a placeholder for the remote render page. This page will be used
        to render a remote project in the browser.
      </p>

      <dialog id="create_job">
        <form method="dialog" onSubmit={handleSubmit}>
          <h1>Dialog</h1>
          <label>Computer Name:</label>
          <input type="text" placeholder="Name" id="name" name="name" />
          <label>Internet Protocol Address</label>
          <input type="text" placeholder="IP Address" id="ip" name="ip" />

          <menu>
            <button type="button" value="cancel" onClick={() => closeCreateNew("create_job")}>Cancel</button>
            <button type="submit">Ok</button>
          </menu>
        </form>
      </dialog>

      <dialog id="create_node">
        <form method="dialog" onSubmit={handleSubmit}>
          <h1>Dialog</h1>
          <label>Computer Name:</label>
          <input type="text" placeholder="Name" id="name" name="name" />
          <label>Internet Protocol Address</label>
          <input type="text" placeholder="IP Address" id="ip" name="ip" />

          <menu>
            <button type="button" value="cancel" onClick={() => closeCreateNew("create_node")}>Cancel</button>
            <button type="submit">Ok</button>
          </menu>
        </form>
      </dialog>
    </div>
  );
}