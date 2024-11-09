import { CiTrash } from "react-icons/ci";
import { MdOutlineMovie } from "react-icons/md";

export interface ProjectFileProps {
  file_name: String;
  path: String;
  onDataChanged?: () => void;
  onRequestNewJob?: (project: ProjectFileProps) => void;
}

// todo: expose function controls here. props event handler?
// No function is referecing this? Do we need this file?
export default function ProjectFile(props: ProjectFileProps) {

  // TODO: From this side of the application - this is really just to delete the entry from the UI element.
  // this should not directly delete the original file. It can send notification to the client node to remove the temp file from storage.
  const deleteProject = () => {
    // TODO: 
    // invoke("delete_project", { projectFile: props }).then(props.onDataChanged);
    if (props.onDataChanged != undefined) {
      props.onDataChanged();
    }
  }

  const createNewJob = () => props.onRequestNewJob ? props.onRequestNewJob(props) : null;

  return (
    <div className="item">
      <table>
        <tbody>
          <tr>
            <td style={{ width: "100%" }}>
              <p>{props.file_name}</p>
            </td>

            {/* <td>
              <MdEdit onClick={editProject} />
            </td> */}
            <td>
              <MdOutlineMovie onClick={createNewJob} />
            </td>
            <td>
              <CiTrash onClick={deleteProject} />
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  );
}
