
import { Link, useNavigate } from 'react-router-dom';
import styled from 'styled-components';
import { Button } from "./Buttons";
import { FiGithub, FiExternalLink, FiPackage, FiBook, FiCode, FiDownload, FiPlay } from 'react-icons/fi';

const PAGEWIDTH = 1300;

const Container = styled.div`
  position: relative;
  background: #433e3c;
  color: #f0e7e4;

  & a {
    text-decoration: none;
    color: inherit;
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
  }

  & a:hover {
    text-decoration: underline;
  }

`;

const MenuContainer = styled.nav`
  display: flex;
  width: 100%;
  align-items: center;
  justify-content: flex-end;
  gap: 1rem;
`;

export const ActionButton = styled(Button)`
    font-size: 1.1rem;
    gap: 0.5rem;
    background: none;
    color: inherit;

    & > * {
      position: relative;
      width: 1.3rem;
      top: 2px;
    }

`;


const Hero = styled.div`
  position: relative;
  width: 100%;
  padding-bottom: 10rem;
  box-sizing: border-box;
  background:  #f1f1f1;
  color: #161413;

  &>div {
    padding: 2rem;
    max-width: ${PAGEWIDTH}px;
    margin-left: auto;
    margin-right: auto;
    box-sizing: border-box;
  }
`;

const Features = styled.div`
  position: relative;
  top: -3rem;
  width: 100%;
  max-width: ${PAGEWIDTH}px;
  margin-left: auto;
  margin-right: auto;
  display: grid;
  padding-left: 1rem;
  padding-right: 1rem;
  gap: 1rem;
  grid-template-columns: [start] 1.5fr [left] 1fr [right] 1fr [end];
  box-sizing: border-box;

  & h1 {
    font-size: 1.5rem;
  }

  & h2 {
    font-size: 1.1rem;
  }
`;


const Logo = styled.div`
  display: flex;
  gap: 1.5rem;
  align-items: center;
  margin-top: 1rem;
  margin-bottom: 2rem;

  & > img {
    width: 7rem;
  }

  & > div > h1 {
    line-height: normal;
    margin: 0;
    font-size: 3.5rem;
  }

  & > div > h2 {
    line-height: normal;
    margin-top: -10px;
    opacity: 0.7;
  }
`;

const About = styled.div`
  font-size: 1.1rem;
  opacity: 0.7;
  max-width: 60ch;
`;

const Footer = styled.footer`
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  width: 100%;
  height: 15rem;

  & > h3 {
    margin-bottom: 1rem;
  }

  & > nav {
    justify-content: center;
  }
`;

const CodeFont = styled.span`
  font-family: 'JetBrains Mono', monospace;
  & > * {
    font-family: 'JetBrains Mono', monospace;
  }
`;

type FeatureProps = {
  color: string,
  action: React.ReactNode,
  children: React.ReactNode,
  gridStart?: GridLine,
  gridEnd?: GridLine,
};

type GridLine = "start" | "left" | "right" | "end";

const FeatureContainer = styled.div<{ color: string, gridStart?: GridLine, gridEnd?: GridLine }>`
display: relative;
padding: 1rem;
box-sizing: border-box;
position: relative;
border-radius: 1rem;
background: ${props => props.color};
color: white;
display: flex;
align-items: top;
justify-content: center;
box-shadow: 0 1px 5px #0000001c;
${props => props.gridStart ? `grid-column-start: ${props.gridStart};` : ""}
${props => props.gridEnd ? `grid-column-end: ${props.gridEnd};` : ""}
`;

const FeatureAction = styled.div`
position: absolute;
bottom: 0;
left:0;
padding: 1.5rem;
font-size: 1.1rem;
box-sizing: border-box;
border-top: 2px dashed #ffffff2f;
width: 100%;
text-align: right;
grid-row-start: left-start;
grid-row-end: end;
z-index: 100;
`;

const FeatureChildren = styled.div`
margin-bottom: 2.5rem;
padding: 1.5rem;
box-sizing: border-box;

& p {
  max-width: 60ch;
}

& img {
  max-width: 100%;
  
}
`;

function Feature({ color, children, action, gridStart, gridEnd }: FeatureProps) {
  return <FeatureContainer color={color} gridStart={gridStart} gridEnd={gridEnd}>
    <FeatureChildren>{children}</FeatureChildren>
    <FeatureAction>{action}</FeatureAction>
  </FeatureContainer>
}




export default function Homepage() {
  const navigate = useNavigate();
  const menu = <MenuContainer>
    <Link to="playground">Playground</Link>
    <Link to="guide">Guide</Link>
    <Link to="package-docs">Package docs</Link>
    <a href="https://github.com/modmark-org/modmark"> GitHub</a>
  </MenuContainer >;

  return <Container>
    <Hero>
      <div>
        {menu}
        <Logo>
          <img src="./logo.svg" alt="ModMark logo" />
          <div>
            <h1>ModMark</h1>
            <h2>Modular Markup Language</h2>
          </div>
        </Logo>

        <About>
          <p>
            ModMark is a modular markup language. It has a lightweight syntax akin to Markdown but also offers a lot more flexibility and expressive power. Import packages and use modules to add extra functionality in your document or even add support for a new output format.
          </p>
          <p>
            ModMark comes bundled with multiple useful packages and can by default output both HTML and LaTeX documents using a web tool or a command-line interface.
          </p>
        </About>
        <ActionButton onClick={(_e) => navigate("/playground")}>Try it in your browser <FiPlay /></ActionButton>
      </div>
    </Hero >
    <Features>
      <Feature
        action={<Link to="/guide">Getting started guide <FiBook /></Link>}
        color="#816796"
      >
        <h1>
          <span style={{ opacity: 0.4 }}>#</span> Familiar and <span style={{ opacity: 0.4 }}>**</span>powerful<span style={{ opacity: 0.4 }}>**</span>
        </h1>
        <p>
          If you have used Markdown before you will quickly feel right at home.
        </p>
        <p>
          Read the getting started guide to learn more about different ways to <CodeFont>**style** ==text==</CodeFont> and include other elements in your document like a <CodeFont>[image](./image.png)</CodeFont> or a <CodeFont>[link](https://modmark.org)</CodeFont>.
        </p>
      </Feature>

      <Feature
        action={<Link to="/package-docs">Explore package docs <FiPackage /> </Link>}
        color="#7392B7"

      >
        <h1>Packages and modules add extra capabilities to your documents.</h1>
        <p>
          Want to add a citations? Plot a math function? Or render a chess board? There is a package for that.
        </p>
      </Feature>

      <Feature
        action={<a href="https://github.com/modmark-org/modmark">Github <FiExternalLink /> </a>}
        color="#C1666B"
      >
        <h1>Free and open source</h1>
        <p>
          The ModMark compiler is free and open source software, learn more on the GitHub page. Feel free to report issues or contribute too.
        </p>
        <FiGithub size={80} style={{ marginTop: "0.3rem", float: "right" }} />
      </Feature>
      <Feature
        action={<a href="">Read developer guide <FiCode /></a>}
        color="#748E54"
        gridStart="start"
        gridEnd="right"
      >
        <h1>
          Develop packages using your favourite language
        </h1>
        <p>
          ModMark packages are <a href="https://webassembly.org/">WebAssembly (WASM)</a> programs that use the <a href="https://wasi.dev/">WebAssembly System Interface (WASI)</a>. This means that you can develop packages using many different languages.
        </p>
        <img width="70%" src="./languages.svg" />

      </Feature>
      <Feature action={<>Download thesis <FiDownload /> </>} color="#0F8B8D">
        <h1>Learn more</h1>
        <p>
          ModMark was created as a bachelor's thesis project at Chalmers University of Technology in Gothenburg, Sweden.
        </p>
      </Feature>
    </Features>
    <Footer>
      <h2>ModMark</h2>
      {menu}
    </Footer>

  </Container >
}