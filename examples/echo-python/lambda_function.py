import json
import logging

logger = logging.getLogger()
logger.setLevel(logging.INFO)

def handler(event, context):
    logger.info("echo event: %s", event)
    return {
        'statusCode': 200,
        'body': json.dumps({
            'ok': True,
            'input': event
        })
    }