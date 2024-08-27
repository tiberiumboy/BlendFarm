import * as GoIcons from "react-icons/go";
import * as Hi2Icon from "react-icons/hi2";
import * as MdIcon from "react-icons/md";
import { Link } from "react-router-dom";
import NodeWindow from "./node_window";

class SidebarStruct {
  public title: string;
  public path: string;
  public icon: any;

  constructor(title: string, path: string, icon: any) {
    this.title = title;
    this.path = path;
    this.icon = icon;
  }
}

const SidebarData = [
  new SidebarStruct("Remote Render", "./remote_render", <GoIcons.GoProject />),
  new SidebarStruct("Setting", "./setting", < Hi2Icon.HiOutlineCog8Tooth />),
  new SidebarStruct("LiveView", "./liveview", < MdIcon.MdOutlinePreview />),
];

export default function Sidebar() {
  return (
    <div className={"sidebar"}>
      <nav >
        <ul className="nav-menu-items">
          {SidebarData.map((item, index) => {
            return (
              <li key={index} className={"nav-bar"}>
                <Link to={item.path}>
                  <span>{item.icon}</span>
                  <span>{item.title}</span>
                </Link>
              </li>
            );
          })}
        </ul>
      </nav>
      {NodeWindow()}
    </div>
  );
}
