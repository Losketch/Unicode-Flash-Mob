import React from "react";
import { useTranslation } from "react-i18next";
import {
  Box,
  Button,
  Card,
  CardContent,
  Link,
  Typography,
} from "@mui/material";
import GitHubIcon from "@mui/icons-material/GitHub";
import { APP_VERSION } from "../utils/config";

const GITHUB_URL = "https://github.com/Losketch/Unicode-Flash-Mob";

const AboutPage: React.FC = () => {
  const { t } = useTranslation("common");

  return (
    <Box
      sx={{
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        minHeight: "100%",
      }}
    >
      <Card elevation={2} sx={{ borderRadius: 3, width: "100%", maxWidth: 420 }}>
        <CardContent sx={{ p: 3, textAlign: "center" }}>
          <Typography variant="h5" gutterBottom sx={{ fontWeight: 600 }}>
            {t("appName")}
          </Typography>
          <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
            {t("version")} {APP_VERSION}
          </Typography>
          <Typography variant="body2" color="text.secondary" sx={{ mb: 3 }}>
            <Link
              href="https://www.apache.org/licenses/LICENSE-2.0"
              target="_blank"
              rel="noopener"
            >
              Apache License 2.0
            </Link>
          </Typography>
          <Button
            variant="contained"
            startIcon={<GitHubIcon />}
            href={GITHUB_URL}
            target="_blank"
            rel="noopener"
          >
            {t("github")}
          </Button>
        </CardContent>
      </Card>
    </Box>
  );
};

export default AboutPage;
