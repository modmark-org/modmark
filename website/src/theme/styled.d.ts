import "styled-components/";
import { ThemeType } from "./ThemeProvider";

declare module "styled-components/" {
  export interface DefaultTheme extends ThemeType {}
}
