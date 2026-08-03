import React from "react";
import { HashRouter, Routes, Route } from "react-router-dom";
import { SnackbarProvider } from "notistack";
import Layout from "./components/Layout";
import StartPage from "./pages/StartPage";
import EditorPage from "./pages/EditorPage";
import TasksPage from "./pages/TasksPage";
import SettingsPage from "./pages/SettingsPage";
import AboutPage from "./pages/AboutPage";
import { AppThemeProvider } from "./contexts/AppThemeProvider";
import { TaskProvider } from "./contexts/TaskContext";
import { WorkflowProvider } from "./contexts/WorkflowContext";

const App: React.FC = () => {
  return (
    <AppThemeProvider>
      <SnackbarProvider
        maxSnack={3}
        anchorOrigin={{ horizontal: "right", vertical: "top" }}
        autoHideDuration={3000}
      >
        <TaskProvider>
          <WorkflowProvider>
            <HashRouter>
            <Layout>
              <Routes>
                <Route path="/" element={<StartPage />} />
                <Route path="/editor" element={<EditorPage />} />
                <Route path="/tasks" element={<TasksPage />} />
                <Route path="/settings" element={<SettingsPage />} />
                <Route path="/about" element={<AboutPage />} />
              </Routes>
            </Layout>
          </HashRouter>
          </WorkflowProvider>
        </TaskProvider>
      </SnackbarProvider>
    </AppThemeProvider>
  );
};

export default App;
