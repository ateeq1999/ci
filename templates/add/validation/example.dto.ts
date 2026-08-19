import { IsEmail, IsNotEmpty } from 'class-validator';

export class ExampleDto {
  @IsEmail()
  email: string;

  @IsNotEmpty()
  password: string;
}
