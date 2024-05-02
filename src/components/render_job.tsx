import { ProjectFileProps } from "../components/project_file";
import { CiCircleMore, CiTrash } from "react-icons/ci";
import { invoke } from "@tauri-apps/api/tauri";

export interface RenderJobProps {
  id: string;
  project_file: ProjectFileProps;
  src?: string;
  picture?: string;
  onDataChanged?: () => void;
}

export default function RenderJob(job: RenderJobProps) {
  const deleteJob = () =>
    invoke("delete_job", { id: job.id }).then(job.onDataChanged);

  const moreAction = () => {
    // should probably provide some context menu?
    console.log("more action was pressed | TODO: add impl.");
  };

  const showCompletedImage = () => {
    if (job.src != null) {
      return <img src={job.src} alt="completed" />;
    } else {
      return <div></div>;
    }
  };

  const showImage = () => {
    if (job.picture != null) {
      return <img src={job.picture} alt="completed" />;
    }
    return <div></div>;
  };

  return (
    <div>
      <table>
        <tbody>
          <tr>
            <td>{job.project_file.file_name}</td>
            <td>{job.project_file.src}</td>
            {showImage()}
            <td>
              b
              <CiTrash onClick={deleteJob} />
            </td>
            <td>
              <CiCircleMore onClick={moreAction} />
            </td>
          </tr>
        </tbody>
      </table>
      {showCompletedImage()}
    </div>
  );
}
