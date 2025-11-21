import "./App.css";
import ServerList from "./components/ServerList";

function App() {
  return (
    <div className="container">
      <div className="main-panel">
        <header>
          <h1>Witch Servers</h1>
        </header>
        <main>
          <ServerList />
        </main>
      </div>
    </div>
  );
}

export default App;
