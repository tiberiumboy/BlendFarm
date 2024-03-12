export type ProjectFileProps = {
  id?: string;
  title: string;
  edit?: Function;
};

// todo: expose function controls here. props event handler?
export default function ProjectFile(props: ProjectFileProps) {
  const handleClick = (e) => {
    if (props.edit) {
      props.edit();
    }
    alert("Editing!");
    return e;
  };

  return (
    <div className="item" id={props.id} onClick={handleClick}>
      {props.title}
    </div>
  );
}
