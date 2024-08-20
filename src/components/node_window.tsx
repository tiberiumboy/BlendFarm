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
    // TODO: I don't like this hacky string cast. It was a solution to make json work, but prefer to do this properly sanitized. Security instinct: Feels unwise to feed arbitruary malicious code injection to the json parser. 
    invoke("list_node").then((ctx: any) => setNodes(JSON.parse(ctx + "")));
  }

  function pingNode() {
    invoke("ping_node");
  }

  function addNode() {
    // try to add the node, then refresh the list
    invoke("add_node").then(() => listNodes());
  }

  function nodeWindow() {
    return (
      <div>
        {/* Show the activity of the computer progress */}
        <h2>Computer Nodes</h2>
        {/* TODO: change this button to send out a ping instead. */}
        <button onClick={pingNode}>Ping</button>
        {/* TODO: Bring up a dialog to prompt the user the IP and port to connect to */}
        {/* <button onClick={addNode}>Connect</button> */}
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

  return (
    <div>
      {/* I'm concern about dialog window size dimension */}
      {nodeWindow()}
    </div>
  );
}
