const _ = require('lodash');
const moment = require('moment');
const { v4: uuidv4 } = require('uuid');
const axios = require('axios');
const validator = require('validator');

exports.handler = async (event) => {
    const { testId, message, input, operation } = event;
    
    console.log('Large dependencies Lambda function invoked');
    console.log('Event:', JSON.stringify(event, null, 2));
    
    try {
        let result;
        
        switch (operation) {
            case 'lodash_test':
                result = {
                    operation: 'lodash_test',
                    result: _.capitalize(input || 'hello world'),
                    timestamp: moment().format('YYYY-MM-DD HH:mm:ss')
                };
                break;
                
            case 'uuid_test':
                result = {
                    operation: 'uuid_test',
                    result: uuidv4(),
                    timestamp: moment().format('YYYY-MM-DD HH:mm:ss')
                };
                break;
                
            case 'validator_test':
                result = {
                    operation: 'validator_test',
                    result: {
                        isEmail: validator.isEmail(input || 'test@example.com'),
                        isURL: validator.isURL(input || 'https://example.com'),
                        isNumeric: validator.isNumeric(input || '123')
                    },
                    timestamp: moment().format('YYYY-MM-DD HH:mm:ss')
                };
                break;
                
            case 'axios_test':
                try {
                    const response = await axios.get('https://httpbin.org/get');
                    result = {
                        operation: 'axios_test',
                        result: {
                            status: response.status,
                            data: response.data.origin
                        },
                        timestamp: moment().format('YYYY-MM-DD HH:mm:ss')
                    };
                } catch (error) {
                    result = {
                        operation: 'axios_test',
                        error: error.message,
                        timestamp: moment().format('YYYY-MM-DD HH:mm:ss')
                    };
                }
                break;
                
            case 'all_deps_test':
                result = {
                    operation: 'all_deps_test',
                    result: {
                        lodash: _.capitalize(input || 'all dependencies test'),
                        uuid: uuidv4(),
                        moment: moment().format('YYYY-MM-DD HH:mm:ss'),
                        validator: {
                            isEmail: validator.isEmail('test@example.com'),
                            isURL: validator.isURL('https://example.com')
                        }
                    },
                    timestamp: moment().format('YYYY-MM-DD HH:mm:ss')
                };
                break;
                
            default:
                result = {
                    operation: 'default',
                    result: {
                        message: message || 'Hello from large dependencies Lambda function',
                        input: input || 'no input provided',
                        testId: testId || 'default',
                        dependencies: {
                            lodash: 'loaded',
                            moment: 'loaded',
                            uuid: 'loaded',
                            axios: 'loaded',
                            validator: 'loaded'
                        }
                    },
                    timestamp: moment().format('YYYY-MM-DD HH:mm:ss')
                };
        }
        
        const response = {
            success: true,
            testId: testId || 'default',
            message: message || 'Hello from large dependencies Lambda function',
            input: input || 'no input provided',
            timestamp: new Date().toISOString(),
            nodeVersion: process.version,
            runtime: 'node',
            operation: operation || 'default',
            result: result,
            validation: {
                allDependenciesLoaded: true,
                lodashWorking: typeof _.capitalize === 'function',
                momentWorking: typeof moment === 'function',
                uuidWorking: typeof uuidv4 === 'function',
                axiosWorking: typeof axios.get === 'function',
                validatorWorking: typeof validator.isEmail === 'function'
            },
            event: event
        };
        
        console.log('Response:', JSON.stringify(response, null, 2));
        
        return response;
        
    } catch (error) {
        console.error('Error in Lambda function:', error);
        
        return {
            success: false,
            error: error.message,
            stack: error.stack,
            testId: testId || 'default',
            timestamp: new Date().toISOString()
        };
    }
};
