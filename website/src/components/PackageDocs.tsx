import { useState } from "react";
import { FiPackage } from "react-icons/fi";
import { MdExpandLess, MdExpandMore } from "react-icons/md";
import styled from "styled-components";
import { PackageInfo, Transform as TransformType } from "../types/compiler";

const PackageContainer = styled.div`
  padding: 0.5rem;
  width: 100%;
  box-sizing: border-box;
`;

const PackageName = styled.h1`
  font-size: 1.5rem;
  line-height: 1.5rem;
  margin: 0;
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
`;

const Version = styled.span`
  font-size: 0.8rem;
  opacity: 0.7;
`;

const PackageHeader = styled.div`
  display: flex;
  gap: 1rem;
  align-items: flex-end;
  justify-content: space-between;
  padding-bottom: 0.5rem;
  border-bottom: solid 2px #0000005e;
`;

const PackageDescription = styled.p`
  font-size: 1rem;
`;

const Subheading = styled.h2`
  margin-top: 1.2rem;
  font-size: 1rem;
  font-family: "Inter", sans-serif;
`;

const PackageDocsContainer = styled.div`
  display: flex;
  flex-direction: column;
  gap: 2rem;
  padding: 1rem;
  background: white;
  width: 100%;
  box-sizing: border-box;

  & code {
    font-family: "JetBrains Mono", monospace;
  }

  & p {
    max-width: 60ch;
  }
`;

const TransformContainer = styled.div`
  border-radius: 0.5rem;
  box-sizing: border-box;
  padding: 0.5rem;
  background: #f8f8f8;
  transition: all 0.2s ease-in-out;
  cursor: pointer;
`;

const From = styled.code`
  font-size: 1.2rem;
`;

const To = styled.div`
  display: flex;
  gap: 0.3rem;
  opacity: 0.7;
`;

const TransformHeading = styled.div`
  display: flex;
  justify-content: space-between;
  align-items: center;
  user-select: none;

  & > div {
    gap: 1rem;
    display: flex;
  }
`;

const TransformList = styled.div`
  & > * {
    margin-top: 0.5rem;
  }
`;

const TransformDetails = styled.div`
  margin-top: 1rem;
`;

const VarName = styled.code`
  font-weight: bold;
`;

const Default = styled.code`
  opacity: 0.7;
`;

const TypeContainer = styled.code<{ color: string }>`
  background: ${(props) => props.color};
  display: inline-flex;
  justify-content: center;
  align-items: center;
  padding: 0.1rem 0.3rem;
  border-radius: 0.3rem;
  color: white;
  font-size: 0.8rem;
`;

const Arg = styled.div`
  border-bottom: solid 1px #0000001c;
  padding: 0.5rem;
`;

function ArgType({ type }: { type: string | string[] }) {
  let str;
  let color;
  if (Array.isArray(type)) {
    str = type.join(" | ");
    color = "#c1666b";
  } else if (type === "String") {
    color = "#748e54";
    str = type;
  } else if (type === "Unsigned integer") {
    color = "#7392b7";
    str = type;
  } else if (type === "Integer") {
    color = "#0f8b8d";
    str = type;
  } else if (type === "Float") {
    color = "#816796";
    str = type;
  }

  return <TypeContainer color={color ?? "#748e54"}>{str}</TypeContainer>;
}

function TransformTypeLabel({ type }: { type: string }) {
  let color = "";
  let text = "";
  switch (type) {
    case "module":
      color = "#831fa4";
      text = "Module";
      break;
    case "inline-module":
      color = "#372cd3";
      text = "Inline-only module";
      break;
    case "multiline-module":
      color = "#bf2ccb";
      text = "Multiline-only module";
      break;
    case "parent":
      color = "#b0962e";
      text = "Parent";
      break;
    case "any":
      color = "#fa6606";
      text = "Any";
      break;
  }

  return <TypeContainer color={color ?? "#123456"}>{text}</TypeContainer>;
}

function Transform({ transform }: { transform: TransformType }) {
  const [expanded, SetExpanded] = useState(false);
  return (
    <TransformContainer onClick={() => SetExpanded((expanded) => !expanded)}>
      <TransformHeading>
        <div>
          {expanded ? (
            <MdExpandLess style={{ marginRight: 5 }} />
          ) : (
            <MdExpandMore style={{ marginRight: 5 }} />
          )}
          <From>{transform.from}</From>
          <TransformTypeLabel type={transform.type} />
        </div>
        <To>
          supports {transform.to.length === 0 && "any"}{" "}
          {transform.to.map((to) => (
            <span key={to}>{to}</span>
          ))}
        </To>
      </TransformHeading>
      {expanded && (
        <TransformDetails>
          {transform.description && <p>{transform.description}</p>}
          {transform.arguments.length === 0 ? (
            <Subheading>No arguments</Subheading>
          ) : (
            <Subheading>Arguments</Subheading>
          )}
          {transform.arguments.map((arg) => (
            <Arg key={arg.name}>
              <div style={{ display: "flex", gap: "1rem", marginBottom: "0.5rem" }}>
                <VarName>{arg.name}</VarName>
                <Default>
                  {arg.default !== null ? `default = ${JSON.stringify(arg.default)}` : "required"}
                </Default>
              </div>

              <ArgType type={arg.type} />
              <p>{arg.description}</p>
            </Arg>
          ))}

          {Object.entries(transform.variables).length === 0 ? (
            <Subheading>No variables</Subheading>
          ) : (
            <Subheading>Variables</Subheading>
          )}
          {Object.entries(transform.variables).map(([name, info]) => (
            <div key={name}>
              <VarName>{name}</VarName>
              <p>Type: {info.type}</p>
              <p>Access: {info.access}</p>
            </div>
          ))}
        </TransformDetails>
      )}
    </TransformContainer>
  );
}

function Package({ pkg }: { pkg: PackageInfo }) {
  return (
    <PackageContainer>
      <PackageHeader>
        <PackageName>
          <FiPackage /> {pkg.name}
        </PackageName>
        <Version>version {pkg.version}</Version>
      </PackageHeader>
      <PackageDescription>{pkg.description}</PackageDescription>
      <Subheading>Transforms</Subheading>
      <TransformList>
        {pkg.transforms.map((transform) => (
          <Transform key={transform.from + transform.to} transform={transform} />
        ))}
      </TransformList>
    </PackageContainer>
  );
}

type PackageDocsProps = {
  packages: PackageInfo[];
};

export default function PackageDocs({ packages }: PackageDocsProps) {
  const packagesElem = packages.map((pkg) => <Package key={pkg.name} pkg={pkg} />);
  return <PackageDocsContainer>{packagesElem}</PackageDocsContainer>;
}
