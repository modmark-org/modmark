import { FC, ReactNode } from "react";
import { ThemeProvider as ThemesProvider } from "styled-components";
import theme from "./theme.json";
import { GlobalStyle } from "./GlobalStyle";

export type ThemeType = typeof theme;

export interface ThemeProviderProps {
  children?: ReactNode;
}

export const ThemeProvider: FC<ThemeProviderProps> = ({ children }) => {
  return (
    <ThemesProvider theme={theme}>
      <GlobalStyle />
      <body>{children}</body>
    </ThemesProvider>
  );
};
