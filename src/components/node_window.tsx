import { invoke } from "@tauri-apps/api/core";
import { RenderNodeProps } from "./render_node";
import { useState } from "react";
import RenderNode from "./render_node";

export default function NodeWindow() {
  // Why is fetchNodes not being called?
  const [nodes, setNodes] = useState(fetchNodes);

  function fetchNodes() {
    const initialNodes: RenderNodeProps[] = [];
    listNodes();
    return initialNodes;
  }

  function listNodes() {
    invoke("list_node").then((ctx: any) => {
      console.log(ctx);
      if (ctx == null) {
        return;
      }

      // TODO: I don't like this hacky string cast. It was a solution to make json work, but prefer to do this properly sanitized. Security instinct: Feels unwise to feed arbitruary malicious code injection to the json parser. 
      const data = JSON.parse(ctx + "");
      data.forEach((node: RenderNodeProps) => { node.onDataChanged = listNodes; })
      setNodes(data);
    });
  }

  function nodeWindow() {
    return (
      <div>
        {/* Show the activity of the computer progress */}
        <h2>Computer Nodes</h2>
        <div className="group" id="RenderNodes">
          {nodes.map((node: RenderNodeProps) => RenderNode(node))}
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
