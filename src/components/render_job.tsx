import { ProjectFileProps } from "../components/project_file";
import { CiCircleMore, CiTrash } from "react-icons/ci";
import { invoke, convertFileSrc } from "@tauri-apps/api/core";

export interface RenderJobProps {
  current_frame: number;
  id: string;
  mode: any;
  output: string;
  project_file: ProjectFileProps;
  renders: string[];
  version: string;
  onDataChanged?: () => void;
}

export default function RenderJob(job: RenderJobProps, callback: (job: RenderJobProps) => void) {
  const deleteJob = () =>
    invoke("delete_job", { targetJob: job }).then(job.onDataChanged);

  const moreAction = () => {
    // should probably provide some context menu?
    console.log("more action was pressed | TODO: add impl.");
  };

  return (
    <div>
      <table>
        <tbody>
          <tr onClick={() => callback(job)}>
            <td style={{ width: "100%" }}>{job.project_file.file_name}</td>
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
