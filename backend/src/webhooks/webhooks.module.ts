import { Module } from '@nestjs/common';
import { TypeOrmModule } from '@nestjs/typeorm';
import { HttpModule } from '@nestjs/axios';
import { WebhookEndpoint } from './entities/webhook-endpoint.entity';
import { WebhookDeliveryLog } from './entities/webhook-delivery-log.entity';
import { WebhooksService } from './services/webhooks.service';
import { WebhookDispatcherService } from './services/webhook-dispatcher.service';
import { WebhookCronService } from './services/webhook-cron.service';
import { WebhooksController } from './webhooks.controller';

@Module({
  imports: [
    TypeOrmModule.forFeature([WebhookEndpoint, WebhookDeliveryLog]),
    HttpModule,
  ],
  controllers: [WebhooksController],
  providers: [WebhooksService, WebhookDispatcherService, WebhookCronService],
  exports: [WebhookDispatcherService],
})
export class WebhooksModule {}
