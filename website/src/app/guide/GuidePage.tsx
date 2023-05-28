import styled from "styled-components";
import Guide from "../../components/Guide";
import { Link } from "react-router-dom";

const Container = styled.div`
  display: flex;
  justify-content: center;
  width: 100%;
  margin-top: 2rem;
`;

export default function GuidePage() {
  return (
    <Container>
      <Link to="../">
        <img src="./logo.svg" alt="Logo" width="80" />
      </Link>
      <Guide />
    </Container>
  );
}
