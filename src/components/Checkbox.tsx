export interface CheckboxProps {
  name: string;
  checked: boolean;
  onChange: (e: React.ChangeEvent<HTMLInputElement>) => void;
}

export default function Checkbox(props: CheckboxProps) {
  <input
    type={"checkbox"}
    name={props.name}
    checked={props.checked}
    onChange={props.onChange}
  />;
}
