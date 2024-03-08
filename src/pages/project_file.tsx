
type ProjectFileProps = {
    id?: string,
    title: string,
    edit?: Function,
}
// todo: expose function controls here. props event handler?
export default function ProjectFile(props: ProjectFileProps) {
    return (
        <div className="item" id={props.id}>
            {props.title}
        </div>
    )
}
