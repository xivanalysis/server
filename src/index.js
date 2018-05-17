import 'dotenv/config'
import Koa from 'koa'

import compress from 'koa-compress'
import cors from '@koa/cors'
import mount from 'koa-mount'
import serve from 'koa-static'

import api from './api'
import proxy from './proxy'

const app = new Koa()

// Shrink that stuff up a bit
app.use(compress())

// Loosen up CORS a bit
app.use(cors())

// Serve the main public assets
// TODO: Would be nice to be able to hit a url without the /#/
app.use(serve(process.env.PUBLIC_PATH || 'public'))

// Mount the fancy stuff
app.use(mount('/api', api))
app.use(mount('/proxy/fflogs', proxy))

// Boot the server
app.listen(process.env.PORT || 3001)
