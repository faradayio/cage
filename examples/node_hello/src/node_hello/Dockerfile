# Use the standard node images.
FROM node

WORKDIR /app

# We manage our dependencies with yarn, so install that first.
RUN npm install -g yarn

# In order to improve build times, we first add package.json and run `yarn
# install` to get our dependencies, then we add the rest of our source
# code.  This allows Docker to cache the results of `yarn install` and not
# re-run it every time
ADD package.json .
RUN yarn install
ADD ./ .

CMD ["node", "app.js"]
