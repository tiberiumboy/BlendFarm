import { invoke } from "@tauri-apps/api/core";
import { CiTrash, CiCircleMore } from "react-icons/ci";

export interface RenderNodeProps {
  name: string;
  spec?: ComputerSpec;
}

export type ComputerSpec = {
  host: string;
  cpu: string;
  gpu?: string;
  arch: string;
  os: string;
  cores: number;
  memory: number;
}

export default function RenderNode(node: RenderNodeProps) {
  return (
    <div key={"Node_" + node.name}>
      <table>
        <tbody>
          <tr>
            <td style={{ width: "100%" }}>
              {node.spec?.host ?? "Loading"}
              <p>{node.spec?.core ?? ""}</p>
              <p>{node.spec?.gpu ?? ""}</p>
              <div>{node.spec?.os ?? ""}</div>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  );
}
