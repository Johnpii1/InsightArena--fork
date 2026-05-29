import {
  Controller,
  Get,
  Patch,
  Delete,
  Param,
  Query,
  Body,
  HttpCode,
  HttpStatus,
} from '@nestjs/common';
import {
  ApiTags,
  ApiOperation,
  ApiResponse,
  ApiBearerAuth,
  ApiQuery,
} from '@nestjs/swagger';
import { NotificationsService } from './notifications.service';
import { UsersService } from '../users/users.service';
import { CurrentUser } from '../common/decorators/current-user.decorator';
import { User } from '../users/entities/user.entity';
import { Notification } from './entities/notification.entity';

@ApiTags('Notifications')
@ApiBearerAuth()
@Controller('notifications')
export class NotificationsController {
  constructor(
    private readonly notificationsService: NotificationsService,
    private readonly usersService: UsersService,
  ) {}

  @Get()
  @ApiOperation({ summary: 'Get notifications for authenticated user' })
  @ApiQuery({ name: 'page', required: false, type: Number })
  @ApiQuery({ name: 'limit', required: false, type: Number })
  @ApiQuery({ name: 'unread_only', required: false, type: Boolean })
  @ApiResponse({ status: 200, description: 'Paginated notifications list' })
  async getMyNotifications(
    @CurrentUser() user: User,
    @Query('page') page = 1,
    @Query('limit') limit = 20,
    @Query('unread_only') unreadOnly?: string,
  ) {
    return this.notificationsService.findAllForUser(
      user.id,
      Number(page),
      Number(limit),
      unreadOnly === 'true',
    );
  }

  @Patch(':id/read')
  @HttpCode(HttpStatus.NO_CONTENT)
  @ApiOperation({ summary: 'Mark a notification as read' })
  @ApiResponse({ status: 204, description: 'Marked as read' })
  async markAsRead(
    @Param('id') id: string,
    @CurrentUser() user: User,
  ): Promise<void> {
    return this.notificationsService.markAsRead(id, user.id);
  }

  @Patch('read-all')
  @HttpCode(HttpStatus.OK)
  @ApiOperation({ summary: 'Mark all notifications as read' })
  @ApiResponse({ status: 200, description: 'Count of notifications updated' })
  async markAllAsRead(@CurrentUser() user: User): Promise<{ updated: number }> {
    return this.notificationsService.markAllAsRead(user.id);
  }

  @Patch(':address/read')
  @HttpCode(HttpStatus.OK)
  @ApiOperation({ summary: 'Mark notifications as read by user address' })
  @ApiResponse({ status: 200, description: 'Count of notifications updated' })
  async markAsReadByAddress(
    @Param('address') address: string,
    @Body() body: { notificationIds?: string[]; markAll?: boolean },
    @CurrentUser() user: User,
  ): Promise<{ updated: number }> {
    const targetUser = await this.usersService.findByAddress(address);
    if (!targetUser || targetUser.id !== user.id) {
      return { updated: 0 };
    }

    if (body.markAll) {
      return this.notificationsService.markAllAsRead(user.id);
    }

    if (body.notificationIds && body.notificationIds.length > 0) {
      return this.notificationsService.markMultipleAsRead(
        user.id,
        body.notificationIds,
      );
    }

    return { updated: 0 };
  }

  @Delete(':id')
  @HttpCode(HttpStatus.NO_CONTENT)
  @ApiOperation({ summary: 'Delete a notification' })
  @ApiResponse({ status: 204, description: 'Notification deleted' })
  async remove(
    @Param('id') id: string,
    @CurrentUser() user: User,
  ): Promise<void> {
    return this.notificationsService.remove(id, user.id);
  }
}
