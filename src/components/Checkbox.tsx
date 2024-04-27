export interface CheckboxProps {
  id?: String,
  name?: string;
  checked: boolean;
  value?: any;
  onDataChanged: (e: React.ChangeEvent<HTMLInputElement>, props: any) => void;
  // onChange: (e: React.ChangeEvent<HTMLInputElement>) => void;
}

export default function Checkbox(props: CheckboxProps) {

  function onChange(e: React.ChangeEvent<HTMLInputElement>) {
    props.onDataChanged(e, props.value);
  }
  return (
    <div key={props.id + "_" + props.name}>
      <label>{props.name}</label>
      <input
        type={"checkbox"}
        name={props.name}
        checked={props.checked}
        onChange={onChange}
      />
    </div>
  )
}
