/* 0040daaf FUN_0040daaf */

void __cdecl FUN_0040daaf(char *param_1)

{
  size_t sVar1;
  int local_c;
  
  local_c = 0;
  while( true ) {
    sVar1 = _strlen(param_1);
    if ((int)sVar1 <= local_c) break;
    if (param_1[local_c] == '\r') {
      param_1[local_c] = ' ';
    }
    local_c = local_c + 1;
  }
  return;
}
