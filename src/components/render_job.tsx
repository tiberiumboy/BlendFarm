import { CiCircleMore, CiTrash } from "react-icons/ci";
import { invoke } from "@tauri-apps/api/core";

export interface RenderJobProps {
  current_frame: number;
  id: string;
  start_frame: number,
  end_frame: number,
  output: string;
  project_file: string;
  renders: string[];
  version: string;
  onDataChanged?: () => void;
}

export function GetFileName(project_file: string) {
  return project_file.split("\\").pop()?.split("/").pop()
} 

export default function RenderJob(job: RenderJobProps, callback: (job: RenderJobProps) => void) {
  const deleteJob = () =>
    invoke("delete_job", { targetJob: job }).then(job.onDataChanged);

  const moreAction = () => {
    // Display context menu here - Currently there are none, so we'll leave this alone for now.
    // console.log("more action was pressed | TODO: add impl.");
  };

  return (
    <div>
      <table>
        <tbody>
          <tr onClick={() => callback(job)}>
            <td style={{ width: "100%" }}>{GetFileName(job.project_file)}</td>
            <td>
              <CiTrash onClick={deleteJob} />
            </td>
            <td>
              <CiCircleMore onClick={moreAction} />
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  );
}
