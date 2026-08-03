import React, { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useLocation, useNavigate } from "react-router-dom";
import {
  AppBar,
  Box,
  Drawer,
  IconButton,
  List,
  ListItem,
  ListItemButton,
  ListItemIcon,
  ListItemText,
  Toolbar,
  Typography,
  Tooltip,
  type Theme,
} from "@mui/material";
import HomeIcon from "@mui/icons-material/Home";
import EditIcon from "@mui/icons-material/Edit";
import TaskIcon from "@mui/icons-material/Task";
import SettingsIcon from "@mui/icons-material/Settings";
import InfoIcon from "@mui/icons-material/Info";
import MenuIcon from "@mui/icons-material/Menu";
import ThemeToggle from "./ThemeToggle";
import { APP_VERSION } from "../utils/config";
import { loadStoredJson, saveStoredJson } from "../utils/storage";

const DRAWER_WIDTH = 220;
const DRAWER_RAIL_WIDTH = 58;
const SIDEBAR_STORAGE_KEY = "ufm-sidebar-expanded";

interface LayoutProps {
  children: React.ReactNode;
}

const Layout: React.FC<LayoutProps> = ({ children }) => {
  const { t } = useTranslation(["common", "app"]);
  const navigate = useNavigate();
  const location = useLocation();
  const [mobileOpen, setMobileOpen] = useState(false);
  const [sidebarExpanded, setSidebarExpanded] = useState(() => {
    const stored = loadStoredJson<unknown>(SIDEBAR_STORAGE_KEY, true);
    return typeof stored === "boolean" ? stored : true;
  });

  useEffect(() => {
    saveStoredJson(SIDEBAR_STORAGE_KEY, sidebarExpanded);
  }, [sidebarExpanded]);

  const navItems = useMemo(
    () => [
      { key: "home", path: "/", icon: <HomeIcon /> },
      { key: "editor", path: "/editor", icon: <EditIcon /> },
      { key: "tasks", path: "/tasks", icon: <TaskIcon /> },
      { key: "settings", path: "/settings", icon: <SettingsIcon /> },
      { key: "about", path: "/about", icon: <InfoIcon /> },
    ],
    []
  );

  const currentNavItem = navItems.find(
    (item) => item.path === location.pathname
  );
  const drawerWidth = sidebarExpanded ? DRAWER_WIDTH : DRAWER_RAIL_WIDTH;
  const drawerPaperTransition = (theme: Theme) =>
    `${theme.transitions.create(["width", "transform"], {
      easing: theme.transitions.easing.sharp,
      duration: theme.transitions.duration.standard,
    })} !important`;

  const drawerContent = (
    <Box sx={{ height: "100%", display: "flex", flexDirection: "column", overflowX: "hidden" }}>
      <Toolbar
        variant="dense"
        sx={{
          minHeight: 48,
          position: "relative",
          p: 0,
        }}
      >
        <Tooltip title={sidebarExpanded ? t("common:collapseSidebar") : t("common:expandSidebar")} placement="right">
          <IconButton
            size="small"
            onClick={() => setSidebarExpanded((expanded) => !expanded)}
            aria-label={sidebarExpanded ? t("common:collapseSidebar") : t("common:expandSidebar")}
            sx={{
              position: "absolute",
              left: 11,
              top: 6,
              width: 36,
              height: 36,
              flex: "0 0 36px",
              transition: "background-color 150ms ease, color 150ms ease",
            }}
          >
            <MenuIcon fontSize="small" />
          </IconButton>
        </Tooltip>
      </Toolbar>
      <List dense sx={{ flexGrow: 1, p: 0 }}>
        {navItems.map((item) => {
          const active = location.pathname === item.path;
          return (
            <ListItem key={item.key} disablePadding sx={{ mb: 0.25 }}>
              <ListItemButton
                dense
                selected={active}
                onClick={() => {
                  navigate(item.path);
                  setMobileOpen(false);
                }}
                sx={{
                  minHeight: 36,
                  mx: 1,
                  p: 0,
                  borderRadius: 1.5,
                }}
              >
                <ListItemIcon
                  sx={{
                    color: active ? "primary.main" : "text.secondary",
                    width: 40,
                    minWidth: 40,
                    flex: "0 0 40px",
                    justifyContent: "center",
                    m: 0,
                  }}
                >
                  {React.cloneElement(item.icon, { fontSize: "small" })}
                </ListItemIcon>
                <ListItemText
                  primary={t(`app:drawer.${item.key}`)}
                  sx={{
                    opacity: sidebarExpanded ? 1 : 0,
                    width: sidebarExpanded ? "auto" : 0,
                    ml: 1,
                    overflow: "hidden",
                    whiteSpace: "nowrap",
                    transition: (theme) => theme.transitions.create(["opacity", "width"], { duration: theme.transitions.duration.standard }),
                  }}
                  primaryTypographyProps={{
                    variant: "body2",
                    fontWeight: active ? 600 : 400,
                    color: active ? "primary.main" : "text.primary",
                  }}
                />
              </ListItemButton>
            </ListItem>
          );
        })}
      </List>
      <Box sx={{ p: 1, opacity: sidebarExpanded ? 1 : 0, transition: "opacity 180ms ease", overflow: "hidden" }}>
        <Typography variant="caption" color="text.secondary" display="block" noWrap>
          v{APP_VERSION}
        </Typography>
      </Box>
    </Box>
  );

  return (
    <Box sx={{ display: "flex", minHeight: "100vh" }}>
      <AppBar
        position="fixed"
        elevation={0}
        sx={{
          width: { xs: "100%", md: `calc(100% - ${drawerWidth}px)` },
          ml: { md: `${drawerWidth}px` },
          transition: (theme) => `${theme.transitions.create(["width", "margin"], { duration: theme.transitions.duration.standard })}`,
        }}
      >
        <Toolbar variant="dense" sx={{ minHeight: 48 }}>
          <IconButton
            color="inherit"
            edge="start"
            size="small"
            onClick={() => setMobileOpen((open) => !open)}
            sx={{ mr: 1, display: { md: "none" } }}
          >
            <MenuIcon fontSize="small" />
          </IconButton>
          <Typography variant="subtitle1" component="div" sx={{ flexGrow: 1, fontWeight: 600 }}>
            {currentNavItem ? t(`app:drawer.${currentNavItem.key}`) : t("app:drawer.home")}
          </Typography>
          <Tooltip title={t("common:theme")}>
            <Box>
              <ThemeToggle />
            </Box>
          </Tooltip>
        </Toolbar>
      </AppBar>

      <Box
        component="nav"
        sx={{
          width: { md: drawerWidth },
          flexShrink: { md: 0 },
          transition: (theme) => theme.transitions.create("width", { duration: theme.transitions.duration.standard }),
        }}
      >
        <Drawer
          variant="temporary"
          open={mobileOpen}
          onClose={() => setMobileOpen(false)}
          ModalProps={{ keepMounted: true }}
          sx={{
            display: { xs: "block", md: "none" },
            "& .MuiDrawer-paper": {
              boxSizing: "border-box",
              width: drawerWidth,
              transition: (theme) => drawerPaperTransition(theme),
            },
          }}
        >
          {drawerContent}
        </Drawer>
        <Drawer
          variant="permanent"
          sx={{
            display: { xs: "none", md: "block" },
            "& .MuiDrawer-paper": {
              boxSizing: "border-box",
              width: drawerWidth,
              overflowX: "hidden",
              transition: (theme) => drawerPaperTransition(theme),
            },
          }}
        >
          {drawerContent}
        </Drawer>
      </Box>

      <Box
        component="main"
        sx={{
          flexGrow: 1,
          p: 1.5,
          pt: { xs: 7, md: 7 },
          width: { md: `calc(100% - ${drawerWidth}px)` },
          transition: (theme) => theme.transitions.create(["width"], { duration: theme.transitions.duration.standard }),
        }}
      >
        {children}
      </Box>
    </Box>
  );
};

export default Layout;
