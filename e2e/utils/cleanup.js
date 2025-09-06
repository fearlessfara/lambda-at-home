#!/usr/bin/env node

/**
 * Cleanup Utility - Clean up test functions and containers
 */

const TestClient = require('../tests/utils/test-client');
const DockerUtils = require('../tests/utils/docker-utils');

async function cleanup() {
    console.log('🧹 Lambda@Home Test Cleanup Utility');
    console.log('===================================');
    console.log('');

    const client = new TestClient();

    try {
        // Check server health
        console.log('🔍 Checking server health...');
        const health = await client.healthCheck();
        if (!health.healthy) {
            console.log('❌ Server is not healthy, skipping function cleanup');
            return;
        }
        console.log('✅ Server is healthy');

        // List and delete all test functions
        console.log('📋 Listing functions...');
        const functions = await client.listFunctions();
        
        if (functions.functions && functions.functions.length > 0) {
            console.log(`📊 Found ${functions.functions.length} functions`);
            
            const testFunctions = functions.functions.filter(f => 
                f.function_name.includes('test-') || 
                f.function_name.includes('service-') ||
                f.function_name.includes('metrics-') ||
                f.function_name.includes('runtime-') ||
                f.function_name.includes('node18-') ||
                f.function_name.includes('node22-') ||
                f.function_name.includes('apigw-') ||
                f.function_name.includes('autoscale-') ||
                f.function_name.includes('concurrency-')
            );

            if (testFunctions.length > 0) {
                console.log(`🗑️  Deleting ${testFunctions.length} test functions...`);
                
                for (const func of testFunctions) {
                    try {
                        await client.deleteFunction(func.function_name);
                        console.log(`✅ Deleted: ${func.function_name}`);
                    } catch (error) {
                        console.log(`❌ Failed to delete ${func.function_name}: ${error.message}`);
                    }
                }
            } else {
                console.log('ℹ️  No test functions found');
            }
        } else {
            console.log('ℹ️  No functions found');
        }

        // Clean up Docker containers
        console.log('🐳 Checking Docker containers...');
        const lambdaContainers = DockerUtils.getLambdaContainers();
        
        if (lambdaContainers.length > 0) {
            console.log(`📊 Found ${lambdaContainers.length} Lambda containers`);
            
            for (const container of lambdaContainers) {
                console.log(`📦 Container: ${container.name} - ${container.status}`);
            }
            
            console.log('ℹ️  Containers will be cleaned up automatically by Lambda@Home');
        } else {
            console.log('ℹ️  No Lambda containers found');
        }

        console.log('');
        console.log('✅ Cleanup completed successfully');

    } catch (error) {
        console.error('❌ Cleanup failed:', error.message);
        process.exit(1);
    }
}

// Run if called directly
if (require.main === module) {
    cleanup().catch(error => {
        console.error('❌ Cleanup failed:', error.message);
        process.exit(1);
    });
}

module.exports = cleanup;
