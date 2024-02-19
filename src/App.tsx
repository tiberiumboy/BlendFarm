// import { invoke } from "@tauri-apps/api/tauri";
import "./App.css";
import "./styles.css";
import Sidebar from "./components/Sidebar";
import { BrowserRouter as Router, Routes, Route } from 'react-router-dom';
import Project from "./pages/project";
import Connection from "./pages/connection";
import Setting from "./pages/setting";

function App() {
  return (
    <div>
      <Router>
        <Sidebar />
        {/* Why switch doesn't exist? */}
        <Routes>
          <Route path='/' Component={Project} />
          <Route path='/project' Component={Project} />
          <Route path='/connection' Component={Connection} />
          <Route path='/setting' Component={Setting} />
        </Routes>
      </Router>
    </div>
  );
}

export default App;
