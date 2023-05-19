import ReactDOM from 'react-dom/client'
import {createHashRouter, Outlet, RouterProvider,} from "react-router-dom";
import Playground from "./Playground";
import Homepage from './Homepage.tsx'
import "normalize.css";
import "./main.css"
import styled from "styled-components";
import {useState} from 'react';
import {Button} from './Buttons.tsx';
import PackageDocsPage from './PackageDocsPage.tsx';
import GuidePage from './GuidePage.tsx';

const DebugMessage = styled.div<{ bg?: string, color?: string }>`
  height: 3rem;
  padding-left: 1rem;
  padding-right: 1rem;
  display: flex;
  align-items: center;
  justify-content: space-between;
  background: ${(props) => props.bg ?? "#e2e2e2"};
  z-index: 1000;
  color: ${(props) => props.color ?? "inherit"};

  & a {
    color: inherit;
  }
`;

function PrPrompt() {
    const [hidden, setHidden] = useState(false);

    const location = window.location;
    const regex = /pr-preview\/pr-(\d+)/;
    const match = location.href.match(regex);

    const showPrompt = !hidden && match !== null;

    return <>
        {showPrompt &&
            <DebugMessage>
                <a href={`https://github.com/modmark-org/modmark/pull/${match[1]}`}>Preview of PR #{match[1]}</a>
                <Button onClick={() => setHidden(true)}>Close</Button>
            </ DebugMessage>
        }
        <Outlet context={showPrompt}/>
    </>
}

// TODO: replace this with a browser router once we have replaced ace and have a proper server
// it would also be possible to only have the playground as a react SPA and use another static site for the rest
const router = createHashRouter([
    {
        path: "/",
        element: <><PrPrompt/></>,
        children: [
            {
                path: "/",
                element: <Homepage/>,
            },
            {
                path: "/playground",
                element: <Playground/>,
            },
            {
                path: "/package-docs",
                element: <PackageDocsPage/>,
            },
            {
                path: "/guide",
                element: <GuidePage/>,
            },
        ],
    },


]);

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
    <RouterProvider router={router}/>
)
