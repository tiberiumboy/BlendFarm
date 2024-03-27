import { invoke } from "@tauri-apps/api/tauri";
import { useEffect, useState } from "react";
import RenderNode, { RenderNodeProps } from "../components/render_node";

function Connection() {
  const [collection, setCollection] = useState([]);

  useEffect(() => {
    listNode();
  }, []);

  async function handleSubmit(e: any) {
    e.preventDefault();
    let data = {
      ip: e.target.ip.value,
      port: Number(e.target.port.value),
    };
    invoke("create_node", data).then((ctx) => {
      listNode();
    });

    return false;
  }

  function listNode() {
    invoke("list_node").then((ctx) => {
      let data = JSON.parse(ctx!);
      setCollection(data);
    });
  }

  function deleteNode(id: any) {
    invoke("delete_node", { id }).then(() => {
      let col = collection.filter((node: any) => node.id != id);
      setCollection(col);
    });
  }

  return (
    <div className="content">
      <h3>Connection</h3>
      <form onSubmit={handleSubmit}>
        <label>Internet Protocol Address</label>
        <input type="text" placeholder="IP Address" id="ip" name="ip" />
        <br></br>
        <input type="number" placeholder="Port" id="port" name="port" />
        <button type="submit">Connect</button>
      </form>
      <br></br>
      <h4>
        <div className="group" id="RenderNodes">
          {collection.map((node: RenderNodeProps) => (
            // A bit far fetch here, but can we rename nodes? Or edit it?
            <RenderNode key={node.id} node={node} onDataChanged={listNode} />
          ))}
        </div>
      </h4>
      <h5>We can also add new render node to this entry.</h5>
    </div>
  );
}

export default Connection;
