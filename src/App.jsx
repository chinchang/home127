import "./App.css";
import ServerList from "./components/ServerList";

function App() {
  return (
    <div className="container">
      <header>
        <h1>Local Servers</h1>
      </header>
      <main>
        <ServerList />
      </main>
    </div>
  );
}

export default App;
