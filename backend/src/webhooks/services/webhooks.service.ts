import { Injectable, Logger, NotFoundException } from '@nestjs/common';
import { InjectRepository } from '@nestjs/typeorm';
import { Repository } from 'typeorm';
import * as crypto from 'crypto';
import { WebhookEndpoint } from '../entities/webhook-endpoint.entity';
import { WebhookDeliveryLog } from '../entities/webhook-delivery-log.entity';
import { CreateWebhookEndpointDto } from '../dto/create-webhook-endpoint.dto';
import { UpdateWebhookEndpointDto } from '../dto/update-webhook-endpoint.dto';
import { User } from '../../users/entities/user.entity';

@Injectable()
export class WebhooksService {
  private readonly logger = new Logger(WebhooksService.name);

  constructor(
    @InjectRepository(WebhookEndpoint)
    private readonly endpointRepository: Repository<WebhookEndpoint>,
    @InjectRepository(WebhookDeliveryLog)
    private readonly deliveryLogRepository: Repository<WebhookDeliveryLog>,
  ) {}

  async createEndpoint(
    user: User,
    dto: CreateWebhookEndpointDto,
  ): Promise<WebhookEndpoint> {
    const secretKey = this.generateSecretKey();

    const endpoint = this.endpointRepository.create({
      user,
      url: dto.url,
      event_types: dto.event_types,
      secret_key: secretKey,
      is_active: true,
      failure_count: 0,
    });

    const saved = await this.endpointRepository.save(endpoint);
    this.logger.log(`Created webhook endpoint for user ${user.id}: ${dto.url}`);

    return saved;
  }

  async findEndpointById(id: string, userId: string): Promise<WebhookEndpoint> {
    const endpoint = await this.endpointRepository.findOne({
      where: { id, user: { id: userId } },
    });

    if (!endpoint) {
      throw new NotFoundException('Webhook endpoint not found');
    }

    return endpoint;
  }

  async listEndpoints(userId: string): Promise<WebhookEndpoint[]> {
    return this.endpointRepository.find({
      where: { user: { id: userId } },
      order: { created_at: 'DESC' },
    });
  }

  async updateEndpoint(
    id: string,
    userId: string,
    dto: UpdateWebhookEndpointDto,
  ): Promise<WebhookEndpoint> {
    const endpoint = await this.findEndpointById(id, userId);

    if (dto.url !== undefined) {
      endpoint.url = dto.url;
    }
    if (dto.event_types !== undefined) {
      endpoint.event_types = dto.event_types;
    }
    if (dto.is_active !== undefined) {
      endpoint.is_active = dto.is_active;
    }

    const updated = await this.endpointRepository.save(endpoint);
    this.logger.log(`Updated webhook endpoint ${id}`);

    return updated;
  }

  async deleteEndpoint(id: string, userId: string): Promise<void> {
    const endpoint = await this.findEndpointById(id, userId);
    await this.endpointRepository.remove(endpoint);
    this.logger.log(`Deleted webhook endpoint ${id}`);
  }

  async getDeliveryLogs(
    endpointId: string,
    userId: string,
    limit: number = 50,
    offset: number = 0,
  ): Promise<{ logs: WebhookDeliveryLog[]; total: number }> {
    const endpoint = await this.findEndpointById(endpointId, userId);

    const [logs, total] = await this.deliveryLogRepository.findAndCount({
      where: { endpoint: { id: endpoint.id } },
      order: { created_at: 'DESC' },
      take: limit,
      skip: offset,
    });

    return { logs, total };
  }

  private generateSecretKey(): string {
    return crypto.randomBytes(32).toString('hex');
  }
}
