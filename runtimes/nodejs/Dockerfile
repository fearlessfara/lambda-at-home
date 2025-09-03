# Use Node.js 22 as base image
FROM node:22-slim

# Create app directory
WORKDIR /usr/src/app

# Copy package files
COPY runtimes/nodejs/package*.json ./

# Install dependencies
RUN npm install --only=production

# Copy runtime source
COPY runtimes/nodejs/src ./src/

# Expose runtime API port
EXPOSE 8080

# Start runtime
CMD [ "node", "src/index.js" ]
