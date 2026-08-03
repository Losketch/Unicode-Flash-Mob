import React, { useMemo } from "react";
import {
  createTheme,
  ThemeProvider,
  CssBaseline,
  responsiveFontSizes,
} from "@mui/material";
import { useSettings } from "./SettingsContext";
import { hexToRgb, isHexColor } from "../utils/config";

export const AppThemeProvider: React.FC<{ children: React.ReactNode }> = ({
  children,
}) => {
  const { effectiveMode, settings } = useSettings();

  const theme = useMemo(() => {
    const accent = isHexColor(settings.accentColor)
      ? settings.accentColor
      : "#6750A4";
    const rgb = hexToRgb(accent);
    const isDark = effectiveMode === "dark";

    let theme = createTheme({
      palette: {
        mode: effectiveMode,
        primary: {
          main: accent,
          contrastText: rgb
            ? (rgb.r * 0.299 + rgb.g * 0.587 + rgb.b * 0.114) > 186
              ? "#000000"
              : "#ffffff"
            : "#ffffff",
        },
        secondary: {
          main: isDark ? "#CCC2DC" : "#625B71",
        },
        background: {
          default: isDark ? "#141218" : "#F7F2FA",
          paper: isDark ? "#211F26" : "#FFFBFE",
        },
        divider: isDark ? "rgba(255, 255, 255, 0.12)" : "rgba(73, 69, 79, 0.18)",
      },
      typography: {
        fontFamily:
          '"HarmonyOSSansSC", "SarasaUiSC", system-ui, sans-serif',
        h5: {
          fontWeight: 500,
        },
      },
      shape: {
        borderRadius: 10,
      },
      components: {
        MuiCssBaseline: {
          styleOverrides: {
            html: {
              fontFamily:
                '"HarmonyOSSansSC", "SarasaUiSC", system-ui, sans-serif',
            },
          },
        },
        MuiDrawer: {
          styleOverrides: {
            paper: {
              backgroundImage: "none",
            },
          },
        },
        MuiPaper: {
          styleOverrides: {
            root: {
              ...(isDark ? {} : { border: "1px solid rgba(73, 69, 79, 0.14)" }),
            },
          },
        },
        MuiCard: {
          styleOverrides: {
            root: {
              ...(isDark ? {} : { border: "1px solid rgba(73, 69, 79, 0.16)" }),
            },
          },
        },
        MuiAppBar: {
          styleOverrides: {
            root: {
              backgroundImage: "none",
            },
          },
        },
        MuiButton: {
          defaultProps: {
            size: "small",
          },
        },
        MuiIconButton: {
          defaultProps: {
            size: "small",
          },
        },
        MuiToolbar: {
          defaultProps: {
            variant: "dense",
          },
        },
        MuiList: {
          defaultProps: {
            dense: true,
          },
        },
        MuiListItem: {
          defaultProps: {
            dense: true,
          },
        },
        MuiListItemButton: {
          defaultProps: {
            dense: true,
          },
        },
        MuiChip: {
          defaultProps: {
            size: "small",
          },
        },
        MuiTextField: {
          defaultProps: {
            variant: "filled",
            size: "small",
          },
        },
        MuiFormControl: {
          defaultProps: {
            variant: "filled",
            size: "small",
          },
        },
      },
    });

    theme = responsiveFontSizes(theme);
    return theme;
  }, [effectiveMode, settings.accentColor]);

  return (
    <ThemeProvider theme={theme}>
      <CssBaseline />
      {children}
    </ThemeProvider>
  );
};
