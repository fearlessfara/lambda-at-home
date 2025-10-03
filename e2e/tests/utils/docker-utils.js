/**
 * Docker Utils - Utilities for Docker container management in tests
 */

const { execSync } = require('child_process');

class DockerUtils {
    static getContainerCount(functionName) {
        try {
            const output = execSync(
                `docker ps --format '{{.Names}}' | grep -E "^lambda-${functionName}-" | wc -l | tr -d ' '`,
                { encoding: 'utf8' }
            );
            return parseInt(output.trim()) || 0;
        } catch (error) {
            return 0;
        }
    }

    static getAllContainers() {
        try {
            const output = execSync('docker ps --format "{{.Names}}\t{{.Status}}"', { encoding: 'utf8' });
            return output.trim().split('\n').map(line => {
                const [name, status] = line.split('\t');
                return { name, status };
            });
        } catch (error) {
            return [];
        }
    }

    static getLambdaContainers() {
        try {
            const output = execSync('docker ps --format "{{.Names}}\t{{.Status}}" | grep "^lambda-"', { encoding: 'utf8' });
            if (!output.trim()) {
                return [];
            }
            return output.trim().split('\n').map(line => {
                const [name, status] = line.split('\t');
                return { name, status };
            }).filter(c => c.name);
        } catch (error) {
            return [];
        }
    }

    static getAllLambdaContainers(includeExited = false) {
        try {
            const psFlag = includeExited ? 'ps -a' : 'ps';
            const output = execSync(`docker ${psFlag} --format "{{.Names}}\t{{.Status}}\t{{.ID}}" | grep "^lambda-"`, { encoding: 'utf8' });
            if (!output.trim()) {
                return [];
            }
            return output.trim().split('\n').map(line => {
                const [name, status, id] = line.split('\t');
                return { name, status, id };
            }).filter(c => c.name);
        } catch (error) {
            return [];
        }
    }

    static getLambdaContainersByFunction(functionName, includeExited = false) {
        try {
            const psFlag = includeExited ? 'ps -a' : 'ps';
            const output = execSync(
                `docker ${psFlag} --format "{{.Names}}\t{{.Status}}\t{{.ID}}" | grep "^lambda-${functionName}-"`,
                { encoding: 'utf8' }
            );
            if (!output.trim()) {
                return [];
            }
            return output.trim().split('\n').map(line => {
                const [name, status, id] = line.split('\t');
                return { name, status, id };
            }).filter(c => c.name);
        } catch (error) {
            return [];
        }
    }

    static getTotalLambdaContainerCount(includeExited = false) {
        const containers = this.getAllLambdaContainers(includeExited);
        return containers.length;
    }

    static async waitForContainerCount(functionName, targetCount, timeoutMs = 10000) {
        const startTime = Date.now();
        
        while (Date.now() - startTime < timeoutMs) {
            const currentCount = this.getContainerCount(functionName);
            if (currentCount >= targetCount) {
                return currentCount;
            }
            await new Promise(resolve => setTimeout(resolve, 1000));
        }
        
        return this.getContainerCount(functionName);
    }

    static async waitForContainerCountChange(functionName, initialCount, timeoutMs = 10000) {
        const startTime = Date.now();
        
        while (Date.now() - startTime < timeoutMs) {
            const currentCount = this.getContainerCount(functionName);
            if (currentCount !== initialCount) {
                return currentCount;
            }
            await new Promise(resolve => setTimeout(resolve, 1000));
        }
        
        return this.getContainerCount(functionName);
    }

    static getContainerLogs(containerName) {
        try {
            const output = execSync(`docker logs ${containerName}`, { encoding: 'utf8' });
            return output;
        } catch (error) {
            return `Failed to get logs: ${error.message}`;
        }
    }

    static getContainerStats(containerName) {
        try {
            const output = execSync(`docker stats ${containerName} --no-stream --format "table {{.CPUPerc}}\t{{.MemUsage}}\t{{.MemPerc}}"`, { encoding: 'utf8' });
            return output;
        } catch (error) {
            return `Failed to get stats: ${error.message}`;
        }
    }
}

module.exports = DockerUtils;
