import styled from "styled-components";

export const Button = styled.button<{ active?: boolean }>`
  display: flex;
  align-items: center;
  gap: 0.2rem;
  border: none;
  box-shadow: 0 0 2px rgba(0, 0, 0, 0.501);
  border-radius: 0.3rem;
  padding: 0.5rem 1rem 0.5rem 1rem;
  background-color: rgb(231, 231, 231);
  transition: 0.2s all;
  color: black;
  font-size: 0.9rem;
  cursor: pointer;

  ${(props) =>
    props.active &&
    `
        background-color: rgb(219, 219, 219);
        box-shadow: 0 0 1px rgba(0, 0, 0, 0.637);`}

  &:hover {
    background-color: rgb(206, 206, 206);
  }
`;

export const Input = styled.input`
  display: flex;
  align-items: center;
  gap: 0.2rem;
  border: none;
  box-shadow: 0 0 2px rgba(0, 0, 0, 0.501);
  border-radius: 0.3rem;
  padding: 0.5rem 1rem 0.5rem 1rem;
  background-color: rgb(231, 231, 231);
  transition: 0.2s background-color;
  color: black;
  font-family: "Inter", sans-serif;
  font-size: 0.9rem;
`;

export const Select = styled.select`
  display: flex;
  align-items: center;
  gap: 0.2rem;
  border: none;
  box-shadow: 0 0 2px rgba(0, 0, 0, 0.501);
  border-radius: 0.3rem;
  padding: 0.5rem 1rem 0.5rem 1rem;
  background-color: rgb(231, 231, 231);
  transition: 0.2s background-color;
  color: black;
  font-family: "Inter", sans-serif;
  font-size: 0.9rem;
`;

export default Button;
