import * as GoIcons from "react-icons/go";
import * as CgIcons from "react-icons/cg";
import * as Hi2Icon from "react-icons/hi2";
import * as MdIcon from "react-icons/md";
import { Link } from "react-router-dom";

export const SidebarData = [
  {
    title: "Remote Render",
    path: "./remote_render",
    icon: <GoIcons.GoProject />,
  },
  {
    title: "Setting",
    path: "./setting",
    icon: <Hi2Icon.HiOutlineCog8Tooth />,
  },
  {
    title: "LiveView",
    path: "./liveview",
    icon: <MdIcon.MdOutlinePreview />,
  },
];

function Sidebar() {
  return (
    <nav className={"sidebar"}>
      {/* Would like to be able to filter the list below with a search box somehow? */}
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
  );
}

export default Sidebar;