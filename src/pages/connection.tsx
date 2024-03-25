import { invoke } from "@tauri-apps/api/tauri";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";

const unlisten = await listen("list_node", (event) => {
  var data = JSON.parse(event.payload);
  console.log(data);
  setNode(data);
});

function Connection() {
  const [node, setNode] = useState([]);

  useEffect(() => {
    listNode();
  }, []);

  async function handleSubmit(e: any) {
    e.preventDefault();
    let data = {
      ip: e.target.ip.value,
      port: Number(e.target.port.value),
    };
    await invoke("create_node", data);
    listNode();
    return false;
  }

  function listNode() {
    invoke("list_node");
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
        Here we will show all of the render client we have previously connected
        to
      </h4>
      <h5>We can also add new render node to this entry.</h5>
    </div>
  );
}

export default Connection;
