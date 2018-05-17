import Redis from 'ioredis'

// Just splitting this out so I don't end up duping connections
const redis = new Redis(process.env.REDIS_DSN)

export default redis
