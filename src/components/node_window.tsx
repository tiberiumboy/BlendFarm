import { invoke } from "@tauri-apps/api/core";
import { RenderNodeProps } from "./render_node";
import { useState } from "react";
import RenderNode from "./render_node";
import { listen } from "@tauri-apps/api/event";

export default function NodeWindow() {
  // Why is fetchNodes not being called?
  const [nodes, setNodes] = useState(fetchNodes);

  function fetchNodes() {
    const initialNodes: RenderNodeProps[] = [];
    listNodes();
    return initialNodes;
  }

  //TODO: read more into this https://v2.tauri.app/develop/calling-frontend/
  // ok this works. Just need to find a way to subscribe on component start, and then unlisten when deconstruct.
  // listen('node_joined', (event) => {
  //   console.log(event);
  // });

  // listen('node_left', (event) => {
  //   console.log(event);
  // });

  function listNodes() {
    invoke("list_node").then((ctx: any) => {
      if (ctx == null) {
        return;
      }
      console.log(ctx);

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
