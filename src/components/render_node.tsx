import { invoke } from "@tauri-apps/api/core";
import { CiTrash, CiCircleMore } from "react-icons/ci";

export interface RenderNodeProps {
  name?: string;
  host?: string;
  onDataChanged?: () => void;
}

export default function RenderNode(node: RenderNodeProps) {
  const deleteNode = (e: any) =>
    invoke("delete_node", { targetNode: node }).then(() =>
      notifyDataChanged(e),
    ); // then we should signal a refresh somehow?

  const notifyDataChanged = (e: any) => {
    if (node.onDataChanged != null) {
      node.onDataChanged();
    }
  };

  return (
    <div key={"Node_" + node.name}>
      <table>
        <tbody>
          <tr>
            <td style={{ width: "100%" }}>{node.name}</td>
            <td>
              <CiCircleMore />
            </td>
            <td>
              <CiTrash onClick={deleteNode} />
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  );
}
