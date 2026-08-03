import React, { useEffect, useState } from "react";
import {
  IconButton,
  InputAdornment,
  TextField,
  Tooltip,
} from "@mui/material";
import InfoOutlinedIcon from "@mui/icons-material/InfoOutlined";

interface JsonEditorFieldProps {
  label: string;
  value: unknown;
  onCommit: (value: unknown) => void;
  invalidMessage: string;
  rows?: number;
  hint?: string;
}

const formatJson = (value: unknown) => JSON.stringify(value ?? {}, null, 2);

const JsonEditorField: React.FC<JsonEditorFieldProps> = ({
  label,
  value,
  onCommit,
  invalidMessage,
  rows = 4,
  hint,
}) => {
  const [draft, setDraft] = useState(() => formatJson(value));
  const [error, setError] = useState("");

  useEffect(() => {
    setDraft(formatJson(value));
    setError("");
  }, [value]);

  const commit = () => {
    try {
      onCommit(JSON.parse(draft));
      setError("");
    } catch {
      setError(invalidMessage);
    }
  };

  return (
    <TextField
      fullWidth
      multiline
      minRows={rows}
      label={label}
      value={draft}
      onChange={(event) => setDraft(event.target.value)}
      onBlur={commit}
      onKeyDown={(event) => {
        if ((event.ctrlKey || event.metaKey) && event.key === "Enter") commit();
      }}
      error={Boolean(error)}
      helperText={error}
      InputProps={
        hint
          ? {
              endAdornment: (
                <InputAdornment position="end">
                  <Tooltip title={hint} arrow>
                    <IconButton size="small" edge="end" aria-label={hint}>
                      <InfoOutlinedIcon fontSize="small" />
                    </IconButton>
                  </Tooltip>
                </InputAdornment>
              ),
            }
          : undefined
      }
      inputProps={{ spellCheck: false }}
    />
  );
};

export default JsonEditorField;
