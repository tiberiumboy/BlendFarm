import { invoke } from "@tauri-apps/api/tauri";
import { useEffect, useState } from "react";
import RenderNode, { RenderNodeProps } from "../components/render_node";

function Connection() {
  const [collection, setCollection] = useState([]);

  useEffect(() => {
    listNode();
  }, []);

  function handleSubmit(e: any) {
    e.preventDefault();
    let data = {
      name: e.target.name.value,
      host: e.target.ip.value + ":" + e.target.port.value,
    };
    invoke("create_node", data).then(listNode);
    closeCreateNew();
  }

  function listNode() {
    invoke("list_node").then((ctx) => {
      let data = JSON.parse(ctx + "");
      setCollection(data);
    });
  }

  function deleteNode(id: any) {
    invoke("delete_node", { id }).then(() => {
      let col = collection.filter((node: any) => node.id != id);
      setCollection(col);
    });
  }

  function showCreateNew() {
    let dialog = document.getElementById("create_new");
    dialog.showModal();
  }

  function closeCreateNew() {
    let dialog = document.getElementById("create_new");
    dialog.close();
  }

  return (
    <div className="content">
      {/* Will be moving this form inside the dialog soon */}
      <h3>Connection</h3>

      <button onClick={showCreateNew}>Create New</button>
      <br></br>
      <h4>
        <div className="group" id="RenderNodes">
          {collection.map((node: RenderNodeProps) => (
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

      <dialog id="create_new">
        <form method="dialog" onSubmit={handleSubmit}>
          <h1>Dialog</h1>
          <label>Computer Name:</label>
          <input type="text" placeholder="Name" id="name" name="name" />
          <label>Internet Protocol Address</label>
          <input type="text" placeholder="IP Address" id="ip" name="ip" />
          <br></br>
          <input type="number" placeholder="Port" id="port" name="port" />

          <menu>
            <button value="cancel">Cancel</button>
            <button type="submit">Ok</button>
          </menu>
        </form>
      </dialog>
    </div>
  );
}

export default Connection;
