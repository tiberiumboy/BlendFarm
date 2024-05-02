import { invoke } from "@tauri-apps/api";
import { CiTrash } from "react-icons/ci";
// import { FaRegPauseCircle, FaRegPlayCircle } from "react-icons/fa";

export interface RenderNodeProps {
  id: string;
  name?: string;
  onDataChanged?: (e: any, node: RenderNodeProps) => void;
}

export default function RenderNode(index: Number, node: RenderNodeProps) {
  const deleteNode = (e: any) =>
    invoke("delete_node", { id: node.id }).then(() => notifyDataChanged(e)); // then we should signal a refresh somehow?

  const pauseNode = (e: any) =>
    // TODO: send a signal to that node to pause
    invoke("pause_node", { id: node.id }).then(() => notifyDataChanged(e));

  const resumeNode = (e: any) =>
    // TODO: Signal commands to resume job.
    invoke("resume_node", { id: node.id }).then(() => notifyDataChanged(e));

  const notifyDataChanged = (e: any) => {
    if (node.onDataChanged != null) {
      node.onDataChanged(e, node);
    }
  };

  return (
    <div key={index}>
      <table>
        <tbody>
          <tr>
            <td style={{ width: "100%" }}>{node.name}</td>
            <td>
              <CiTrash onClick={deleteNode} />
            </td>
            {/* <td>
              Feature: We could have a halt button here? if the node is running,
              we may want to let the user invoke pause or stop operation?

              <FaRegPauseCircle onClick={pauseNode} />
              <FaRegPlayCircle onClick={resumeNode} />
            </td> */}
          </tr>
        </tbody>
      </table>
    </div>
  );
}
