import { invoke } from "@tauri-apps/api/core";
import { RenderNodeProps } from "./render_node";
import { useState } from "react";
import RenderNode from "./render_node";
import { listen } from "@tauri-apps/api/event";

export default function NodeWindow() {
  const [nodes, setNodes] = useState<String[]>([]);

  //TODO: How can I unsubscribe this?
  listen<string>('node_discover', (event: any) => {
    let tmp = [...nodes];
    let id = event.payload;
    if (!tmp.includes(id)) {
      tmp.push(id);
    }
    setNodes(tmp);
  });

  listen('node_disconnect', (event: any) => {
    let tmp = [...nodes];
    let result = tmp.filter((t) => t == event.payload);
    setNodes(result);
  });

  function nodeWindow() {
    return (
      <div>
        {/* Show the activity of the computer progress */}
        <h2>Computer Nodes</h2>
        <div className="group" id="RenderNodes">
          {nodes.map((node: String) =>
            <div>{node}</div> // todo - find a way to simplify this message down to something simplier. 
          )}
        </div>
      </div>
    );
  }

  return (
    <div>
      {nodeWindow()}
    </div>
  );
}
