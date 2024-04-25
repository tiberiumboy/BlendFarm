import * as ciIcon from "react-icons/ci";
import * as mdIcon from "react-icons/md";

export interface ProjectFileProps {
  id: string;
  file_name: string;
  src: string;
  delete?: Function;
}

// todo: expose function controls here. props event handler?
export default function (props: ProjectFileProps) {
  const getFileName = () => {
    return props.src?.split("/").pop()?.split("\\").pop();
  };

  const deleteProject = (e: any) => {
    e.preventDefault();
    if (props.delete) {
      props.delete(props.id);
    }
  };

  return (
    <div className="item" key={props.id} id={props.id}>
      <table>
        <tr>
          <td style={{ width: "100%" }}>
            <p>{getFileName()}</p>
            <p>{props.src}</p>
          </td>

          {/* <td>
            <mdIcon.MdEdit onClick={editProject} />
          </td> */}

          <td>
            <ciIcon.CiTrash onClick={deleteProject} />
          </td>
        </tr>
      </table>
    </div>
  );
}
