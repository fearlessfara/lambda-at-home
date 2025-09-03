# Use Python 3.12 slim image
FROM python:3.12-slim

# Create app directory
WORKDIR /usr/src/app

# Copy requirements file
COPY runtimes/python-ric/requirements.txt .

# Install dependencies
RUN pip install --no-cache-dir -r requirements.txt

# Copy runtime source
COPY runtimes/python-ric/src ./src/

# Expose runtime API port
EXPOSE 8080

# Start runtime
CMD [ "python", "src/runtime.py" ]
