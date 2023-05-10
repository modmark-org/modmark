# Website
The playground uses Vite and React. To run the playground for local development:
* Install dependencies
  ```
  npm install
  ```

* Build the bindings to the compiler.
   ```
   npm run wasm
   ```

* Build the website to static files
   ```
   chmod +x ./build_playground.sh
   ./build_playground.sh
   ```

 ..or start a local development server
   ```
   npm run dev
   ```