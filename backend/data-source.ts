import { config } from 'dotenv';
import dataSource from './src/config/typeorm.config';

// Load environment variables for TypeORM CLI
config();

export default dataSource;
